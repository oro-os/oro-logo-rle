use byteorder::{LittleEndian, WriteBytesExt};
use compression::prelude::*;

const TOTAL_FRAMES: usize = 24 * 3;
const WIDTH: usize = 256;
const HEIGHT: usize = 256;

/// NOTE: This *can* be set to u8 to get *slightly*
/// NOTE: better byte counts. However, it's not
/// NOTE: enough (IMO) to warrant changing at the
/// NOTE: moment.
/// NOTE:
/// NOTE: In the future, I plan to experiment with a
/// NOTE: bit mask or registers to manage switching
/// NOTE: between 16 and 8 bit command sizes, or maybe
/// NOTE: even packed command sizes.
/// NOTE:
/// NOTE: Feel free to flesh this out and get some
/// NOTE: more exotic approaches going if you'd like,
/// NOTE: as long as the entirety of the decoder is
/// NOTE: constant space complexity.
type Cmd = u16;

#[derive(PartialEq)]
enum Command {
	Skip,
	Draw,
}

macro_rules! emit {
	($result:expr, $state:expr, $count:expr) => {
		if $count > 0 {
			$result.push(match $state {
				(Command::Skip, _) => {
					assert_eq!(($count << 1) >> 1, $count); // Otherwise, count is too high.
					($count << 1) as u16
				}
				(Command::Draw, intensity) => {
					assert!(intensity < 4);
					assert_eq!(($count << 3) >> 3, $count); // Otherwise, count is too high.
					(($count << 3) | ((intensity as Cmd) << 1) | 1) as u16
				}
			});
		}
	};
}

pub fn main() {
	let mut result: Vec<u16> = Vec::new();

	let mut last_bmp: Option<lodepng::Bitmap<lodepng::Grey<u8>>> = None;

	// For sequences of frames > 1, re-encode the first frame again.
	for frame in if TOTAL_FRAMES < 2 {
		(0..0).chain(0..=0)
	} else {
		(0..TOTAL_FRAMES).chain(0..=0)
	} {
		let filename = format!(
			"{}/frames/oro_{:0>5}.png",
			env!("CARGO_MANIFEST_DIR"),
			frame
		);

		let bmp = match lodepng::decode_file(filename, lodepng::ColorType::GREY, 8).unwrap() {
			lodepng::Image::Grey(bmp) => bmp,
			_ => panic!("decode_file() returned type other than Grey"),
		};

		assert_eq!(bmp.width, WIDTH);
		assert_eq!(bmp.height, HEIGHT);
		assert_eq!(bmp.buffer.len(), WIDTH * HEIGHT);

		let mut state = (Command::Skip, 0u8);
		let mut count: Cmd = 0;

		match last_bmp {
			None => {
				state = (Command::Draw, 0u8);

				for pixel in &bmp.buffer {
					// Downsample to 2 bits
					let intensity = (**pixel) >> 6;

					if intensity == state.1 {
						count += 1;
						if count == (Cmd::MAX >> 3) {
							emit!(result, state, count);
							count = 0;
						}
					} else {
						emit!(result, state, count);
						state.1 = intensity;
						count = 1;
					}
				}
			}
			Some(last) => {
				if last.width != bmp.width || last.height != bmp.height {
					panic!(
						"all frames must be the same size; last frame was {}x{}, this frame is {}x{}",
						last.width, last.height, bmp.width, bmp.height
					);
				}

				for (new_pixel, old_pixel) in (bmp.buffer).iter().zip(last.buffer.iter()) {
					// Downsample to 2 bits
					let new_intensity = (**new_pixel) >> 6;
					let old_intensity = (**old_pixel) >> 6;

					// Is there a difference from the last
					// frame?
					#[allow(clippy::collapsible_else_if)]
					if new_intensity == old_intensity {
						if state.0 == Command::Skip {
							count += 1;
							if count == (Cmd::MAX >> 1) {
								emit!(result, state, count);
								count = 0;
							}
						} else {
							emit!(result, state, count);
							state.0 = Command::Skip;
							count = 1;
						}
					} else {
						if state == (Command::Draw, new_intensity) {
							count += 1;
							if count == (Cmd::MAX >> 3) {
								emit!(result, state, count);
								count = 0;
							}
						} else {
							emit!(result, state, count);
							state = (Command::Draw, new_intensity);
							count = 1;
						}
					}
				}
			}
		}

		// Emit any residual command
		emit!(result, state, count);

		// Emit the "end frame" command
		// (we use a direct .push() here since emit!() checks
		// the count, which must be 0 in this case)
		result.push(0);

		// Store this frame to reference the last frame
		last_bmp = Some(bmp);
	}

	// Extract the sizes
	let (frame_width, frame_height) = match &last_bmp {
		Some(last_bmp) => (last_bmp.width, last_bmp.height),
		None => panic!("no frames were processed"),
	};

	// Compress it
	let compressed_bytes = result
		.iter()
		.flat_map(|b16| {
			let mut v = Vec::new();
			v.write_u16::<LittleEndian>(*b16).unwrap();
			v.into_iter()
		})
		.encode(&mut BZip2Encoder::new(9), Action::Finish)
		.collect::<Result<Vec<_>, _>>()
		.unwrap();

	// For debugging purposes, we emit the raw file.
	// NOTE: from_u16 *might* modify the array to convert to the correct endianness.
	std::fs::write(
		format!("{}/oro-logo.bin", std::env::var("OUT_DIR").unwrap()),
		&compressed_bytes,
	)
	.unwrap();

	// Then we generate Rust code.
	let total_values = compressed_bytes.len();
	let mut array = syn::punctuated::Punctuated::<syn::LitInt, syn::Token![,]>::new();
	for i in &compressed_bytes {
		array.push(syn::LitInt::new(
			&i.to_string(),
			proc_macro2::Span::call_site(),
		));
	}
	let rust_code = quote::quote! {
		/// The width of the Oro logo represented by this library
		pub const ORO_LOGO_WIDTH: usize = #frame_width;
		/// The width of the Oro logo represented by this library
		pub const ORO_LOGO_HEIGHT: usize = #frame_height;
		/// The *recommended* frames per second for displaying the oro logo
		pub const ORO_LOGO_FPS: usize = 24; // hardcoded for now
		/// The total number of frames in the Oro logo (not counting the
		/// final "loop frame")
		pub const ORO_LOGO_FRAME_COUNT: usize = #TOTAL_FRAMES;

		const ORO_LOGO_IMG_COMPRESSED: [u8; #total_values] = [ #array ];
	}
	.to_string();

	std::fs::write(
		format!("{}/oro-logo.rs", std::env::var("OUT_DIR").unwrap()),
		rust_code,
	)
	.unwrap();
}
