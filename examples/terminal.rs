#![feature(iter_array_chunks)]

use crossterm::{
	execute,
	terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use oro_logo_rle::{self as oro_logo, Command, OroLogo};
use std::{
	io,
	sync::atomic::{AtomicBool, Ordering},
	thread,
	time::Duration,
};
use tui::{
	backend::CrosstermBackend, buffer::Cell, layout::Rect, style::Color, widgets::Widget, Terminal,
};

struct OroLogoRenderer {
	iter: OroLogo,
	seen_first: bool,
	cells: Vec<Cell>,
}

struct OroLogoFrame(Vec<Cell>);

impl OroLogoRenderer {
	fn new() -> Self {
		assert_eq!(oro_logo::ORO_LOGO_WIDTH, 256);
		assert_eq!(oro_logo::ORO_LOGO_HEIGHT, 256);

		Self {
			iter: OroLogo::new(),
			seen_first: false,
			cells: vec![
				Cell {
					fg: Color::Black,
					bg: Color::Black,
					..Cell::default()
				};
				256 * 256
			],
		}
	}

	fn next_frame(&mut self) -> OroLogoFrame {
		// We skip the first for weird TUI-related reasons...
		// I think I'm doing a "no-no" but I don't care too terribly much
		// to figure out exactly what it is (I think the buffer handed to
		// OroLogoFrame::render() is the screen buffer or something, so the
		// first frame area is the entire terminal, and all further calls
		// overwrite the area... not sure)
		if self.seen_first {
			let mut off = 0usize;
			loop {
				match self.iter.next() {
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
							self.cells[off + (i as usize)].bg = color;
						}
						off += count as usize;
					}
					Some(Command::Skip(count)) => {
						off += count as usize;
					}
				}
			}
		} else {
			self.seen_first = true;
		}

		OroLogoFrame(
			self.cells
				.iter()
				.array_chunks::<256>()
				.step_by(4)
				.flat_map(|row| row.into_iter().step_by(2))
				.cloned()
				.collect::<Vec<_>>(),
		)
	}
}

impl Widget for OroLogoFrame {
	fn render(self, _area: Rect, buf: &mut tui::buffer::Buffer) {
		buf.area.width = 128;
		buf.area.height = 64;
		buf.content = self.0;
	}
}

fn main() -> Result<(), io::Error> {
	static mut SHOULD_TERMINATE: AtomicBool = AtomicBool::new(false);

	ctrlc::set_handler(|| unsafe {
		SHOULD_TERMINATE.store(true, Ordering::SeqCst);
	})
	.expect("Error setting Ctrl-C handler");

	let mut stdout = io::stdout();
	execute!(stdout, EnterAlternateScreen)?;
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;

	let mut logo_renderer = OroLogoRenderer::new();

	while !unsafe { SHOULD_TERMINATE.load(Ordering::SeqCst) } {
		terminal.draw(|f| f.render_widget(logo_renderer.next_frame(), Rect::new(0, 0, 128, 64)))?;
		thread::sleep(Duration::from_millis(
			1000 / (oro_logo::ORO_LOGO_FPS as u64),
		));
	}

	execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
	terminal.show_cursor()?;

	Ok(())
}
