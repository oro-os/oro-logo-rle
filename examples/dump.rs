//! Simply dumps the frames to the terminal as bytes re-scaled to 0..=255.

use oro_logo_rle::{Command, OroLogo, ORO_LOGO_FRAME_COUNT, ORO_LOGO_HEIGHT, ORO_LOGO_WIDTH};
use std::io::{self, Write};

fn main() {
	let mut buffer = [0u8; ORO_LOGO_WIDTH as usize * ORO_LOGO_HEIGHT as usize];
	let mut iter = OroLogo::new();
	let mut stdout = io::stdout();

	for _ in 0..ORO_LOGO_FRAME_COUNT {
		let mut off = 0usize;

		loop {
			match iter.next() {
				None => panic!("Oro logo exhausted commands (shouldn't happen)"),

				Some(Command::End) => break,

				Some(Command::Draw(count, lightness)) => {
					let color = lightness * (255 / 3);

					for i in 0..count {
						buffer[off + (i as usize)] = color;
					}

					off += count as usize;
				}

				Some(Command::Skip(count)) => {
					off += count as usize;
				}
			}
		}

		stdout.write_all(&buffer[..]).unwrap();
	}
}
