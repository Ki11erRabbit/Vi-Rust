use std::{io, sync::mpsc::{Receiver, Sender}, cell::RefCell, rc::Rc};

use crossterm::{terminal, execute, cursor::{SetCursorStyle, MoveTo}};

use crate::{window::Window, pane::Pane};



pub enum EditorMessage {
    NextWindow,
    PrevWindow,
    NewWindow(Option<Rc<RefCell<dyn Pane>>>),
    CloseWindow,
    Quit,
    NthWindow(usize),
}



pub struct Editor {
    windows: Vec<Window>,
    active_window: usize,
    reciever: Receiver<EditorMessage>,
    sender: Sender<EditorMessage>,
}


impl Editor {
    pub fn new() -> Self {
        terminal::enable_raw_mode().expect("Failed to enable raw mode");
        execute!(std::io::stdout(), terminal::EnterAlternateScreen).expect("Failed to enter alternate screen");
        execute!(io::stdout(), SetCursorStyle::BlinkingBlock).expect("Could not set cursor style");

        let (sender, reciever) = std::sync::mpsc::channel();
        
        Self {
            windows: vec![Window::new(sender.clone())],
            active_window: 0,
            reciever,
            sender,
        }
    }

    pub fn open_file(&mut self, path: &str) -> io::Result<()> {
        self.windows[self.active_window].open_file_start(path)
    }

    fn check_messages(&mut self) -> io::Result<()> {
        match self.reciever.try_recv() {
            Ok(message) => {
                match message {
                    EditorMessage::NextWindow => {
                        self.active_window = (self.active_window + 1) % self.windows.len();
                        self.windows[self.active_window].force_refresh_screen()?;
                        Ok(())
                    },
                    EditorMessage::PrevWindow => {
                        self.active_window = self.active_window.saturating_sub(1);
                        self.windows[self.active_window].force_refresh_screen()?;
                        Ok(())
                    },
                    EditorMessage::NewWindow(pane) => {
                        self.windows.push(Window::new(self.sender.clone()));
                        self.active_window = self.windows.len() - 1;
                        if let Some(pane) = pane {
                            self.windows[self.active_window].replace_pane(0, pane);
                        }
                        self.windows[self.active_window].force_refresh_screen()?;
                        eprintln!("New window");
                        Ok(())
                    },
                    EditorMessage::CloseWindow => {
                        self.windows.remove(self.active_window);
                        self.active_window = self.active_window.saturating_sub(1);
                        Ok(())
                   },
                    EditorMessage::Quit => {
                        self.windows.clear();
                        Ok(())
                    },
                    EditorMessage::NthWindow(n) => {
                        if n < self.windows.len() {
                            self.active_window = n;
                        }
                        self.windows[self.active_window].force_refresh_screen()?;
                        Ok(())
                    },
                }
            },
            Err(_) => Ok(()),
        }
    }

    pub fn run(&mut self) -> io::Result<bool> {
        self.check_messages()?;

        if self.windows.is_empty() {
            return Ok(false);
        }
        
        self.windows[self.active_window].run()?;
        Ok(true)
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
