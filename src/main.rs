use std::{io, time::Duration};

use crossterm::{terminal, event::{self, KeyEvent, Event}};
use window::Window;

pub mod window;

struct CleanUp;

impl Drop for CleanUp {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not turn off Raw mode");
    }
}

struct KeyReader {
    duration: Duration,
}

impl KeyReader {
    fn read_key(&self) -> io::Result<KeyEvent> {
        loop {
            if event::poll(self.duration)? {
                if let Event::Key(key) = event::read()? {
                    return Ok(key);
                }
            }
        }
    }
}

struct Editor {
    window: Window,
    key_reader: KeyReader,
}

impl Editor {
    fn new() -> Self {
        let window = Window::new();
        let key_reader = KeyReader {
            duration: Duration::from_millis(100),
        };

        Self {
            window,
            key_reader,
        }
    }

    pub fn open_file(&mut self, filename: &str) -> io::Result<()> {
        self.window.open_file(filename)
    }

    pub fn run(&mut self) -> io::Result<bool> {
        self.window.refresh_screen()?;
        self.process_keypress()
    }

    fn process_keypress(&mut self) -> io::Result<bool> {
        match self.key_reader.read_key()? {
            KeyEvent {
                code: event::KeyCode::Char('q'),
                ..
            } => return Ok(false),
            _ => {}
        }

        Ok(true)
    }

}


fn main() -> io::Result<()> {
    let _cleanup = CleanUp;
    terminal::enable_raw_mode()?;

    let mut editor = Editor::new();

    let filename = std::env::args().nth(1).expect("No file name provided");
    editor.open_file(&filename)?;

    while editor.run()? {}

    Ok(())
}
