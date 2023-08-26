use std::{io, time::Duration};

use crossterm::{terminal, event::{self, KeyEvent, Event}, execute, cursor::SetCursorStyle, cursor::MoveTo};
use window::{Window, Pane, TextPane};

pub mod window;
pub mod mode;
pub mod cursor;
pub mod settings;

struct CleanUp;

impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not turn off Raw mode");
        execute!(std::io::stdout(), terminal::Clear(terminal::ClearType::All)).expect("Could not clear terminal");
        execute!(std::io::stdout(), MoveTo(0, 0)).expect("Could not move cursor to (0, 0)");
        execute!(io::stdout(), SetCursorStyle::DefaultUserShape).expect("Could not reset cursor style");
    }
}


struct Editor {
    window: Window<TextPane>,
}

impl Editor {
    fn new() -> Self {
        execute!(io::stdout(),SetCursorStyle::BlinkingBlock).unwrap();
        let window = Window::new();
        Self {
            window,
        }
    }

    pub fn open_file(&mut self, filename: &str) -> io::Result<()> {
        self.window.open_file(filename)
    }

    pub fn run(&mut self) -> io::Result<bool> {
        self.window.run()
    }

}


fn main() -> io::Result<()> {
    let _cleanup = CleanUp;
    terminal::enable_raw_mode()?;

    let mut editor = Editor::new();

    if let Some(filename) = std::env::args().nth(1) {
        editor.open_file(&filename)?;
    }

    while editor.run()? {}

    Ok(())
}
