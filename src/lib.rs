//! An RLE-like 2-bit greyscale image decoder used for displaying the
//! Oro logo in the kernel.
//!
//! The first "frame" is the base image data. It is draw once at the
//! very start of the sequence. At the end of the sequence, drawing is
//! looped back around to the second frame (offset 1), if it exists.
//!
//! Note that this usually means the first frame is re-encoded at the
//! end of the sequence to have a clean transition back to the second
//! frame.
//!
//! A sequence is one or more frames, one directly after the other.
//! Each frame is a series of one or more commands, always ending with
//! a Skip(u15::MAX) command.
//!
//! All commands are 16 bits. Bit 0 indicates the command - either
//! draw (HIGH) or skip (LOW). When drawing, bits 1 and 2 indicate
//! the intensity, bit 2 being the MSB. The remaining 13 bits are
//! the **pixel count**. When skipping, the remaining 15 bits are
//! the **pixel count** to skip (as in, do not alter the pixel color
//! at all).
//!
//! Skip words with a count of 0 are "end markers". Correlary, if
//! the "raw" u16 == 0x0000, then it's the "end frame" marker.
//!
//! Finally, the entire payload is bzip2 compressed (check build.rs
//! for exact parameters).
//!

// TODO: Switch to LZMA once https://github.com/gendx/lzma-rs/issues/43
// TODO: is closed/handled.

#![no_std]
#![deny(unsafe_code)]
#![feature(iter_array_chunks)]

use compression::prelude::*;
use core::iter::{ArrayChunks, Cloned};

include!(concat!(env!("OUT_DIR"), "/oro-logo.rs"));

/// For each frame, this denotes the "command" for the RLE rasterizer
/// to execute. Note that it's EXTREMELY IMPORTANT the implementation
/// for the RLE rasterizer to double check bounds etc (i.e. written in
/// a memory safe language, such as Rust, or performing the checks each
/// time). This is because these commands could possibly do arbitrary
/// memory execution with a malicious payload in memory under the right
/// circumstances (well, strange circumstances... someone would have to
/// load a custom version of the Oro logo, run it through the RLE, and
/// simultaneously not trust the source of the logo but trust the source
/// of the kernel - if you don't trust the kernel itself, all bets are
/// off to begin with).
///
/// Under normal circumstances, following each command issued by the
/// iterator, in order, for each frame, without resetting the iterator
/// (which itself never resets), *should* result in a well-formed logo
/// display with minimal processing/size overhead. In the event the target
/// buffer is resized or needs to be repainted, you'll have to re-iterate
/// up to the currently running frame from a FRESH iterator.
pub enum Command {
	/// Draw `.0` pixels with the intensity `.1`.
	Draw(u16, u8),
	/// Skip the next `.0` pixels.
	Skip(u16),
	/// The end of the frame; the next command emitted from
	/// the iterator is the first command of the next frame.
	End,
}

struct OroLogoDecoded<I>
where
	I: Iterator<Item = [u8; 2]>,
{
	iter: I,
}

impl<I> Iterator for OroLogoDecoded<I>
where
	I: Iterator<Item = [u8; 2]>,
{
	type Item = Command;

	fn next(&mut self) -> Option<Self::Item> {
		self.iter.next().map(|bytes| {
			let raw = u16::from_le_bytes(bytes);
			if raw == 0 {
				Command::End
			} else if (raw & 1) == 0 {
				Command::Skip(raw >> 1)
			} else {
				Command::Draw(raw >> 3, ((raw >> 1) & 0b11) as u8)
			}
		})
	}
}

trait IntoOroLogoDecoded {
	fn decode_oro_logo(self) -> OroLogoDecoded<Self>
	where
		Self: Sized + Iterator<Item = [u8; 2]>,
	{
		OroLogoDecoded { iter: self }
	}
}

impl<T> IntoOroLogoDecoded for T where T: Sized + Iterator {}

struct BZip2Decoded<I>
where
	I: Sized + Iterator<Item = u8>,
{
	decoder: BZip2Decoder,
	iter: I,
}

impl<I> Iterator for BZip2Decoded<I>
where
	I: Sized + Iterator<Item = u8>,
{
	type Item = u8;

	fn next(&mut self) -> Option<Self::Item> {
		self.decoder.next(&mut self.iter).map(|r| r.unwrap())
	}
}

trait IntoBZip2Decoded {
	fn decode_bzip2(self) -> BZip2Decoded<Self>
	where
		Self: Sized + Iterator<Item = u8>,
	{
		BZip2Decoded {
			decoder: BZip2Decoder::new(),
			iter: self,
		}
	}
}

impl<T> IntoBZip2Decoded for T where T: Iterator {}

/// A singular, stateful Oro Logo command iterator.
/// Iterating over this will yield [Command]s, which
/// instruct a cursor how to draw a 256x256 Oro Logo.
///
/// Note that the first frame of the iterator is a complete
/// re-draw, and **no successive frames perform complete redraws**.
/// Thus, consumers that need to support a full re-paint must
/// track the current frame and re-create the iterator and fast-forward
/// in order to "repaint" with the current frame.
///
/// For module operations, etc. the constant `ORO_LOGO_FRAME_COUNT` is
/// exposed. For recommended FPS, use `ORO_LOGO_FPS`. For future-proofing,
/// It's recommended to either assert or somehow gracefully handle
/// different values of `ORO_LOGO_WIDTH` and `ORO_LOGO_HEIGHT`.
pub struct OroLogo {
	decomp: OroLogoDecoded<ArrayChunks<BZip2Decoded<Cloned<core::slice::Iter<'static, u8>>>, 2>>,
	frame_count: usize,
}

impl OroLogo {
	pub fn new() -> Self {
		Self {
			decomp: ORO_LOGO_IMG_COMPRESSED
				.iter()
				.cloned()
				.decode_bzip2()
				.array_chunks::<2>()
				.decode_oro_logo(),
			frame_count: 0,
		}
	}
}

impl Default for OroLogo {
	fn default() -> Self {
		Self::new()
	}
}

impl Iterator for OroLogo {
	type Item = Command;

	fn next(&mut self) -> Option<Self::Item> {
		match self.decomp.next() {
			Some(Command::End) => {
				self.frame_count += 1;
				Some(Command::End)
			}
			None => {
				self.decomp = ORO_LOGO_IMG_COMPRESSED
					.iter()
					.cloned()
					.decode_bzip2()
					.array_chunks::<2>()
					.decode_oro_logo();

				if self.frame_count > 1 {
					// fast forward to frame two
					loop {
						match self.decomp.next() {
							None => panic!(),
							Some(Command::End) => break,
							_ => {}
						}
					}
				}
				self.frame_count = 0;

				let r = self.decomp.next();
				debug_assert!(r.is_some());
				r
			}
			cmd => cmd,
		}
	}
}
