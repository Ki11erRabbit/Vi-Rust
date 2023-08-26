use std::io;

use buffer::RopeBuffer;
use crossterm::{terminal::{self, EnterAlternateScreen}, execute, cursor::SetCursorStyle};
use window::Window;

pub mod window;
pub mod mode;
pub mod cursor;
pub mod settings;
pub mod buffer;

struct CleanUp;

impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not turn off Raw mode");
        Window::clear_screen().expect("Could not clear screen");
        execute!(io::stdout(), SetCursorStyle::DefaultUserShape).expect("Could not reset cursor style");
        execute!(io::stdout(), terminal::LeaveAlternateScreen).expect("Could not leave alternate screen");
    }
}


struct Editor {
    window: Window<RopeBuffer>,
}

impl Editor {
    fn new() -> Self {
        execute!(io::stdout(), EnterAlternateScreen).unwrap();
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
