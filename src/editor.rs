use std::{io, sync::mpsc::{Receiver, Sender}, cell::RefCell, rc::Rc, thread};

use crossterm::{terminal, execute, cursor::{SetCursorStyle, MoveTo}};

use crate::{window::Window, pane::Pane, lsp::{ControllerMessage, LspController}};



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
    lsp_listener: Rc<Receiver<ControllerMessage>>,
    lsp_responder: Sender<ControllerMessage>,
}


impl Editor {
    pub fn new() -> Self {
        terminal::enable_raw_mode().expect("Failed to enable raw mode");
        execute!(std::io::stdout(), terminal::EnterAlternateScreen).expect("Failed to enter alternate screen");
        execute!(io::stdout(), SetCursorStyle::BlinkingBlock).expect("Could not set cursor style");

        let (sender, reciever) = std::sync::mpsc::channel();

        let mut controller = LspController::new();

        let (lsp_sender, lsp_reciever) = std::sync::mpsc::channel();
        let (lsp_controller, lsp_controller_reciever) = std::sync::mpsc::channel();

        controller.set_listener(lsp_reciever);
        controller.set_response(lsp_controller);


        thread::spawn(move || {
            controller.run();
        });

        let lsp_listener = Rc::new(lsp_controller_reciever);
        
        Self {
            windows: vec![Window::new(sender.clone(), lsp_sender.clone(), lsp_controller.clone())],
            active_window: 0,
            reciever,
            sender,
            lsp_listener,
            lsp_responder: lsp_sender,
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
                        self.windows.push(Window::new(self.sender.clone(), self.lsp_listener.clone(), self.lsp_responder.clone()));
                        self.active_window = self.windows.len() - 1;
                        if let Some(pane) = pane {
                            self.windows[self.active_window].replace_pane(0, pane);
                            //eprintln!("New window with pane");
                        }
                        self.windows[self.active_window].force_refresh_screen()?;
                        //eprintln!("New window");
                        Ok(())
                    },
                    EditorMessage::CloseWindow => {
                        //eprintln!("Close window");
                        self.windows.remove(self.active_window);
                        self.active_window = self.active_window.saturating_sub(1);
                        Ok(())
                   },
                    EditorMessage::Quit => {
                        //eprintln!("Quit");
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
            //eprintln!("No windows left, quitting");
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
