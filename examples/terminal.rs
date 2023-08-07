#![feature(iter_array_chunks)]

use crossterm::{
	execute,
	terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use oro_logo_rle::{Command, OroLogo, OroLogoData};
use std::{
	io,
	sync::atomic::{AtomicBool, Ordering},
	thread,
	time::Duration,
};
use tui::{
	backend::CrosstermBackend, buffer::Cell, layout::Rect, style::Color, widgets::Widget, Terminal,
};

#[cfg(feature = "oro-logo-1024")]
type OroLogoSized = oro_logo_rle::OroLogo1024x1024;
#[cfg(feature = "oro-logo-512")]
type OroLogoSized = oro_logo_rle::OroLogo512x512;
#[cfg(feature = "oro-logo-256")]
type OroLogoSized = oro_logo_rle::OroLogo256x256;
#[cfg(feature = "oro-logo-64")]
type OroLogoSized = oro_logo_rle::OroLogo64x64;
#[cfg(feature = "oro-logo-32")]
type OroLogoSized = oro_logo_rle::OroLogo32x32;

type OroLogoImpl = OroLogo<OroLogoSized>;

struct OroLogoRenderer {
	iter: OroLogoImpl,
	seen_first: bool,
	cells: Vec<Cell>,
}

struct OroLogoFrame(Vec<Cell>);

impl OroLogoRenderer {
	fn new() -> Self {
		Self {
			iter: OroLogoImpl::new(),
			seen_first: false,
			cells: vec![
				Cell {
					fg: Color::Black,
					bg: Color::Black,
					..Cell::default()
				};
				OroLogoImpl::WIDTH * OroLogoImpl::HEIGHT
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

		OroLogoFrame(self.cells.clone())
	}
}

impl Widget for OroLogoFrame {
	fn render(self, _area: Rect, buf: &mut tui::buffer::Buffer) {
		buf.area.width = OroLogoImpl::WIDTH as u16;
		buf.area.height = OroLogoImpl::HEIGHT as u16;
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
		terminal.draw(|f| {
			f.render_widget(
				logo_renderer.next_frame(),
				Rect::new(0, 0, OroLogoImpl::WIDTH as u16, OroLogoImpl::HEIGHT as u16),
			)
		})?;
		thread::sleep(Duration::from_millis(1000 / (OroLogoImpl::FPS as u64)));
	}

	execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
	terminal.show_cursor()?;

	Ok(())
}
