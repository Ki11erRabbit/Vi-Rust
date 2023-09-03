use std::io;

use crossterm::{terminal, execute, cursor::{SetCursorStyle, MoveTo}};

use crate::window::Window;







pub struct Editor {
    windows: Vec<Window>,
    active_window: usize,
}


impl Editor {
    pub fn new() -> Self {
        terminal::enable_raw_mode().expect("Failed to enable raw mode");
        execute!(std::io::stdout(), terminal::EnterAlternateScreen).expect("Failed to enter alternate screen");
        execute!(io::stdout(), SetCursorStyle::BlinkingBlock).expect("Could not set cursor style");
        
        Self {
            windows: vec![Window::new()],
            active_window: 0,
        }
    }

    pub fn open_file(&mut self, path: &str) -> io::Result<()> {
        self.windows[self.active_window].open_file_start(path)
    }

    pub fn run(&mut self) -> io::Result<bool> {
        self.windows[self.active_window].run()
    }


}


impl Drop for Editor {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Failed to disable raw mode");
        execute!(std::io::stdout(), terminal::Clear(terminal::ClearType::All)).expect("Failed to clear terminal");
        execute!(std::io::stdout(), MoveTo(0, 0)).expect("Failed to move cursor to 0, 0");
        execute!(std::io::stdout(), terminal::LeaveAlternateScreen).expect("Failed to leave alternate screen");
        execute!(io::stdout(), SetCursorStyle::DefaultUserShape).expect("Could not reset cursor style");
    }
}
