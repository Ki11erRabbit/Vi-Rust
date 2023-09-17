use std::{io, sync::mpsc::{Receiver, Sender}, cell::RefCell, rc::Rc, thread, time::Duration};

use crossterm::{terminal, execute, cursor::{SetCursorStyle, MoveTo}, event::{Event, self}};

use crate::{window::{Window, WindowMessage}, pane::Pane, lsp::{LspControllerMessage, LspController}, registers::{Registers, RegisterUtils}, Mailbox, settings::Settings};



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


pub struct EditorMailbox {
    /// The receiver for messages to the editor
    local_receiver: Receiver<EditorMessage>,
    /// The sender for messages to the editor
    /// This isn't wrapped in an Rc because it is easy to share
    far_sender: Sender<EditorMessage>,
    /// The receiver for messages not to the editor
    /// This is wrapped in an Rc so that it can be shared with other parts of the editor
    far_receiver: Rc<Receiver<EditorMessage>>,
    /// The sender for messages not to the editor
    local_sender: Sender<EditorMessage>,
}

impl EditorMailbox {
    pub fn new() -> Self {
        let (far_sender, local_receiver) = std::sync::mpsc::channel();
        let (local_sender , far_receiver) = std::sync::mpsc::channel();

        Self {
            local_receiver,
            far_sender,
            far_receiver: Rc::new(far_receiver),
            local_sender,
        }
    }

    pub fn get_far_receiver(&self) -> Rc<Receiver<EditorMessage>> {
        self.far_receiver.clone()
    }

    pub fn get_far_sender(&self) -> Sender<EditorMessage> {
        self.far_sender.clone()
    }

}

impl Mailbox<EditorMessage> for EditorMailbox {
    fn send(&self, message: EditorMessage) -> Result<(), std::sync::mpsc::SendError<EditorMessage>> {
        self.local_sender.send(message)
    }
    

    fn recv(&self) -> Result<EditorMessage, std::sync::mpsc::RecvError> {
        self.local_receiver.recv()
    }

    fn try_recv(&self) -> Result<EditorMessage, std::sync::mpsc::TryRecvError> {
        self.local_receiver.try_recv()
    }
}



pub struct Editor {
    windows: Vec<Window>,
    window_senders: Vec<Sender<WindowMessage>>,
    active_window: usize,
    mailbox: EditorMailbox,
    /// The receiver for messages from the LSP controller
    /// It is wrapped in an Rc so that it can be shared with other parts of the editor
    lsp_listener: Rc<Receiver<LspControllerMessage>>,
    /// The sender for messages to the LSP controller
    /// This isn't wrapped in an Rc because it is easy to share
    lsp_sender: Sender<LspControllerMessage>,

    settings: Rc<RefCell<Settings>>,

    poll_duration: Duration,

    registers: Registers,
}


impl Editor {
    pub fn new(lsp_sender: Sender<LspControllerMessage>, lsp_listener: Rc<Receiver<LspControllerMessage>>) -> Self {
        terminal::enable_raw_mode().expect("Failed to enable raw mode");
        execute!(std::io::stdout(), terminal::EnterAlternateScreen).expect("Failed to enter alternate screen");
        execute!(std::io::stdout(), SetCursorStyle::BlinkingBlock).expect("Failed to set cursor style");


        let size = terminal::size()
            .map(|(w, h)| (w as usize, h as usize)).unwrap();

        let mailbox = EditorMailbox::new();


        //todo: load settings from file
        let mut settings = Settings::default();

        settings.cols = size.0;
        settings.rows = size.1;

        let poll_duration = Duration::from_millis(settings.editor_settings.poll_duration);



        let settings = Rc::new(RefCell::new(settings));

        let window = Window::new(mailbox.get_far_sender(),
                                 mailbox.get_far_receiver(),
                                 lsp_sender.clone(),
                                 lsp_listener.clone(),
                                 settings.clone());


        let window_sender = window.get_sender();


        Self {
            windows: vec![window],
            window_senders: vec![window_sender],
            active_window: 0,
            mailbox,
            lsp_listener,
            lsp_sender,
            registers: Registers::new(),
            settings,
            poll_duration,
        }
    }

    pub fn open_file(&mut self, path: &str) -> io::Result<()> {
        self.windows[self.active_window].open_file_start(path)
    }

    fn check_messages(&mut self) -> io::Result<()> {
        match self.mailbox.try_recv() {
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
                        self.windows.push(Window::new(self.mailbox.get_far_sender(),
                                 self.mailbox.get_far_receiver(),
                                 self.lsp_sender.clone(),
                                 self.lsp_listener.clone(),
                                 self.settings.clone()));
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

                                let response = WindowMessage::PasteResponse(response);

                                self.window_senders[self.active_window].send(response).expect("Failed to send paste response");

                                Ok(())
                            },
                            RegisterType::Number(n) => {

                                let text = self.registers.get(n);

                                let response = text.and_then(|text| Some(text.clone().into_boxed_str()));

                                let response = WindowMessage::PasteResponse(response);

                                self.window_senders[self.active_window].send(response).expect("Failed to send paste response");

                                Ok(())
                            },
                            RegisterType::Name(name) => {

                                let text = self.registers.get(name);

                                let response = text.and_then(|text| Some(text.clone().into_boxed_str()));

                                let response = WindowMessage::PasteResponse(response);

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

    fn resize(&mut self, size: (usize, usize)) -> io::Result<()> {
        for window in &mut self.windows {
            window.resize(size.0, size.1);
        }
        Ok(())
    }

    fn process_event(&mut self) -> io::Result<Event> {
        //self.refresh_screen()?;
        loop {
            if event::poll(self.poll_duration)? {
                return event::read();
            }
        }
    }

    pub fn draw(&mut self) -> io::Result<()> {
        self.windows[self.active_window].refresh_screen()?;
        self.windows[self.active_window].reset_active_pane();
        
        Ok(())
    }


    pub fn run(&mut self) -> io::Result<bool> {
        self.check_messages()?;

        if !self.windows[self.active_window].can_continue()? {
            self.windows.remove(self.active_window);
            self.active_window = self.active_window.saturating_sub(1);
        }

        if self.windows.is_empty() {
            //eprintln!("No windows left, quitting");
            return Ok(false);
        }

        self.windows[self.active_window].refresh_screen()?;


        self.windows[self.active_window].reset_active_pane();

        let event = self.process_event()?;
        match event {
            Event::Key(key) => {
                if self.windows[self.active_window].skip_event() {
                    return Ok(true);
                }

                self.windows[self.active_window].process_keypress(key)?;
                Ok(true)
            },
            Event::Resize(width, height) => {
                self.resize((width as usize, height as usize))?;

                Ok(true)
            },
            _ => {
                //eprintln!("Unhandled event: {:?}", event);
                Ok(true)
            },

        }
        
        //self.windows[self.active_window].run()?;
        //Ok(true)
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
