//! Simply dumps the frames to the terminal as bytes re-scaled to 0..=255.

use oro_logo_rle::{Command, OroLogo, OroLogo256x256, OroLogoData};
use std::io::{self, Write};

fn main() {
	let mut buffer = [0u8; OroLogo256x256::WIDTH * OroLogo256x256::HEIGHT];
	let mut iter = OroLogo::<OroLogo256x256>::new();
	let mut stdout = io::stdout();

	for _ in 0..OroLogo256x256::FRAMES {
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
