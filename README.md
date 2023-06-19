<center>
<img src="screenshot.gif" alt="An animated screenshot of the Oro logo displayed in the command line" />
</center>
<br />

This houses the RLE-like image format decoder for progressive animation of the Oro logo
in constrained environments, such as kernels and embedded devices.

A singular, concrete, non-generic iterator type is exposed (`OroLogo`) that, when iterated,
gives an endless stream of cursor commands useful for continuously updating a persistent buffer
of pixel information.

The format supports up to 4 lightness levels (`0..=3`) that can be used for
a primitive amount of anti-aliasing, and are supported by many OLED/LCD displays alike.

The iterator itself performs a single base draw on the first frame, followed by an endless looping
stream of update frames that modify the previously drawn frame directly. This includes the frame
that 'wraps around' to the first frame, avoiding expensive redraws when the animation loops (however
there is still a small decompression cost upon looping).

The library is optimized for cases where individual pixel updates are expensive (e.g. direct-to-framebuffer
rasterizer implementations, such as those found in the Oro kernel, or bus-issued pixel updates, such as those
over SPI or I<sup>2</sup>C lines).

The entire logo animation, as of June 20, 2023, fits in about 23KiB of static storage.

# Usage

You can see a (mostly cross-platform) in-terminal example by running


```shell
cargo run --example terminal
```

Implementing the decoder for a buffer of 256x256 _linear_ pixels should be
as simple as the following:

```rust
use oro_logo_rle::{
	OroLogo, Command,
	ORO_LOGO_WIDTH, ORO_LOGO_HEIGHT,
	ORO_LOGO_FPS,
	/* ORO_LOGO_FRAME_COUNT */
};

let mut iter = OroLogo::new();

// (uses fictional `Color` type)
let mut buffer = [Color; ORO_LOGO_WIDTH * ORO_LOGO_HEIGHT];

loop {
	let mut off = 0usize;

	match iter.next() {
		None => panic!("Oro logo exhausted commands (shouldn't happen)"),

		Some(Command::End) => break,

		Some(Command::Draw(count, lightness)) => {
			let color = match lightness {
				0 => Color::Black,
				1 => Color::DarkGray,
				2 => Color::Gray,
				3 => Color::White,
				_ => unreachable!(),
			};

			for i in 0..count {
				buffer[off + (i as usize)] = color;
			}

			off += count as usize;
		}

		Some(Command::Skip(count)) => {
			off += count as usize;
		}
	}

	// (fictional per-frame operations)
	preset(&buffer);
	sleep_ms(1000 / ORO_LOGO_FPS);
}
```

# License
Copyright &copy; 2023, Joshua Lee Junon.

A license is to be determined. Please do not use code in this repository in any fashion until one is issued.
