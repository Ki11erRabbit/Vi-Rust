use std::{io, sync::mpsc::{Receiver, Sender}, cell::RefCell, rc::Rc, thread};

use crossterm::{terminal, execute, cursor::{SetCursorStyle, MoveTo}};

use crate::{window::{Window, Message}, pane::Pane, lsp::{ControllerMessage, LspController}, registers::{Registers, RegisterUtils}};



pub enum EditorMessage {
    NextWindow,
    PrevWindow,
    NewWindow(Option<Rc<RefCell<dyn Pane>>>),
    CloseWindow,
    Quit,
    NthWindow(usize),
    Paste(RegisterType),
    Copy(RegisterType, String),
}

#[derive(Clone, Debug)]
pub enum RegisterType {
    Number(usize),
    Name(String),
    None,
}

pub struct Editor {
    windows: Vec<Window>,
    window_senders: Vec<Sender<Message>>,
    active_window: usize,
    reciever: Receiver<EditorMessage>,
    sender: Sender<EditorMessage>,
    lsp_listener: Rc<Receiver<ControllerMessage>>,
    lsp_responder: Sender<ControllerMessage>,

    registers: Registers,
}


impl Editor {
    pub fn new(lsp_sender: Sender<ControllerMessage>, lsp_listener: Rc<Receiver<ControllerMessage>>) -> Self {
        terminal::enable_raw_mode().expect("Failed to enable raw mode");
        execute!(std::io::stdout(), terminal::EnterAlternateScreen).expect("Failed to enter alternate screen");
        execute!(io::stdout(), SetCursorStyle::BlinkingBlock).expect("Could not set cursor style");


        let (sender, reciever) = std::sync::mpsc::channel();


        //eprintln!("Editor created");
        //let lsp_listener = Rc::new(lsp_controller_reciever);

        let window = Window::new(sender.clone(), lsp_sender.clone(), lsp_listener.clone());

        let window_sender = window.get_sender();
        
        Self {
            windows: vec![window],
            window_senders: vec![window_sender],
            active_window: 0,
            reciever,
            sender,
            lsp_listener,
            lsp_responder: lsp_sender,
            registers: Registers::new(),
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
                        self.windows.push(Window::new(self.sender.clone(), self.lsp_responder.clone(), self.lsp_listener.clone()));
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
                    EditorMessage::Paste(ty) => {
                        match ty {
                            RegisterType::None => {

                                eprintln!("Pasting from clipboard");

                                let text = self.registers.get_clipboard();

                                eprintln!("{:?}", text);

                                let response = text.and_then(|text| Some(text.into_boxed_str()));

                                let response = Message::PasteResponse(response);

                                self.window_senders[self.active_window].send(response).expect("Failed to send paste response");

                                Ok(())
                            },
                            RegisterType::Number(n) => {

                                let text = self.registers.get(n);

                                let response = text.and_then(|text| Some(text.clone().into_boxed_str()));

                                let response = Message::PasteResponse(response);

                                self.window_senders[self.active_window].send(response).expect("Failed to send paste response");

                                Ok(())
                            },
                            RegisterType::Name(name) => {

                                let text = self.registers.get(name);

                                let response = text.and_then(|text| Some(text.clone().into_boxed_str()));

                                let response = Message::PasteResponse(response);

                                self.window_senders[self.active_window].send(response).expect("Failed to send paste response");

                                Ok(())
                            },
                                               
                        }
                    },
                    EditorMessage::Copy(ty, text) => {
                        match ty {
                            RegisterType::None => {
                                self.registers.set_clipboard(text);
                                Ok(())
                            },
                            RegisterType::Number(n) => {
                                self.registers.set(n, text);
                                Ok(())
                            },
                            RegisterType::Name(name) => {
                                self.registers.set(name, text);
                                Ok(())
                            },
                        }
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
