use std::{collections::HashMap, sync::mpsc::{Sender, Receiver}, io, process::{Command, Stdio}, task::Poll};

use self::lsp_client::LspClient;

pub mod lsp_client;


pub enum LspRequest {
    /// Tells the server to shutdown
    Shutdown,
    /// Tells the server to exit
    Exit,

}

pub enum LspResponse {

}

pub enum LspNotification {
    /// 0 is the uri
    /// 1 is the version
    /// 2 is the text
    ChangeText(Box<str>, usize, Box<str>),
    /// 0 is the uri
    /// 1 is the text
    Open(Box<str>, Box<str>),
    /// 0 is the uri
    Close(Box<str>),
    /// 0 is the uri
    /// 1 is the text
    Save(Box<str>, Box<str>),
    /// 0 is the uri
    /// 1 is the reason
    WillSave(Box<str>, Box<str>),


}

pub enum ControllerMessage {
    /// String is the language id
    Request(Box<str>, LspRequest),
    /// Box<str> is the language id
    Response(Box<str>, LspResponse),
    /// Box<str> is the language id
    Notification(Box<str>, LspNotification),
    /// String is the language id
    CreateClient(Box<str>),
    /// Notification to tell the caller how to recieve responses
    /// The receiver is for the language server side
    ClientCreated(Receiver<ControllerMessage>),
    /// Notification to tell the caller that there is no client for the language
    NoClient,
    
    

}


pub struct LspController {
    clients: HashMap<String, Box<dyn LspClient>>,
    channels: (Sender<ControllerMessage>, Receiver<ControllerMessage>),
    server_channels: HashMap<String, Sender<ControllerMessage>>,
}



impl LspController {

    pub fn new() -> Self {
        LspController {
            clients: HashMap::new(),
            channels: std::sync::mpsc::channel(),
            server_channels: HashMap::new(),
        }
    }



    pub fn run(&mut self) -> io::Result<()> {
        loop {
            self.check_messages()?;

            for (_, client) in self.clients.iter_mut() {
                let json = client.process_messages()?;
                match json.poll() {
                    Ok(Poll::Ready(json)) => {
                        json.wake();
                    },
                    Err(Poll::Ready(err)) => {
                        return Err(io::Error::new(io::ErrorKind::Other, "Error processing messages"));
                    },
                    Ok(Poll::Pending) | Err(Poll::Pending) => {
                        continue;
                    },
                }
            }
        }
    }


    fn check_messages(&mut self) -> io::Result<()> {
       
        match self.channels.1.try_recv() {
            Ok(ControllerMessage::CreateClient(lang)) => {
                self.create_client(lang)
            },
            Ok(ControllerMessage::Request(lang, req)) => {
                self.check_request(lang, req)
            },
            Ok(ControllerMessage::Notification(lang, notif)) => {
                self.check_notification(lang, notif)
            },
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                return Err(io::Error::new(io::ErrorKind::Other, "Channel disconnected"));
            }
            Err(_) => {
                return Ok(());
            },
            _ => {
                return Ok(());
            }
        }

    }

    fn check_notification<R>(&mut self, lang: R, notif: LspNotification) -> io::Result<()> where R: AsRef<str> {
        match self.clients.get_mut(&*lang) {
            Some(client) => {
                match notif {
                    LspNotification::ChangeText(uri, version, text) => {
                        client.did_change_text(uri.as_ref(), version, text.as_ref())?;
                    },
                    LspNotification::Open(uri, text) => {
                        client.did_open_text(uri, text)?;
                    },
                    LspNotification::Close(uri) => {
                        client.send_did_close_text(uri)?;
                    },
                    LspNotification::Save(uri, text) => {
                        client.did_save_text(uri.as_ref(), text.as_ref())?;
                    },
                    LspNotification::WillSave(uri, reason) => {
                        let reason = match reason.as_ref() {
                            "manual" => 1,
                            "afterDelay" => 2,
                            "focusOut" => 3,
                            _ => {
                                return Err(io::Error::new(io::ErrorKind::Other, "Invalid reason"));
                            }
                        };
                        
                        client.will_save_text(uri.as_ref(), reason)?;
                    },
                }
            },
            None => {
                return Err(io::Error::new(io::ErrorKind::Other, "No client for language"));
            }
        }

        Ok(())
    }

    fn check_request(&mut self, lang: Box<str>, req: LspRequest) -> io::Result<()> {
        match self.clients.get_mut(&*lang) {
            Some(client) => {
                match req {
                    LspRequest::Shutdown => {
                        client.send_shutdown()?;
                    },
                    LspRequest::Exit => {
                        client.send_exit()?;
                    },
                }
            },
            None => {
                return Err(io::Error::new(io::ErrorKind::Other, "No client for language"));
            }
        }

        Ok(())
    }

    fn create_client<R>(&mut self, lang: R) -> io::Result<()> where R: AsRef<str> {
        let client = match lang {
            "rust" => {
                let rust_analyzer = Command::new("rust-analyzer")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::Client::new(rust_analyzer.stdin.unwrap(), rust_analyzer.stdout.unwrap());

                lsp_client.initialize()?;

                Box::new(lsp_client) as Box<dyn LspClient>
            },
            "c" | "cpp" => {
                let clangd = Command::new("clangd")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::LspClient::new(clangd.stdin.unwrap(), clangd.stdout.unwrap());

                lsp_client.initialize()?;

                Box::new(lsp_client) as Box<dyn LspClient>
            },
            "python" => {
                let python_lsp = Command::new("python-lsp-server")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::LspClient::new(python_lsp.stdin.unwrap(), python_lsp.stdout.unwrap());

                lsp_client.initialize()?;

                Box::new(lsp_client) as Box<dyn LspClient>
            },
            "swift" => {
                let apple_swift = Command::new("sourcekit-lsp")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::LspClient::new(apple_swift.stdin.unwrap(), apple_swift.stdout.unwrap());

                lsp_client.initialize()?;

                Box::new(lsp_client) as Box<dyn LspClient>
            },
            "go" => {
                let gopls = Command::new("gopls")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::LspClient::new(gopls.stdin.unwrap(), gopls.stdout.unwrap());

                lsp_client.initialize()?;

                Box::new(lsp_client) as Box<dyn LspClient>
            },
            "bash" => {
                let bash_lsp = Command::new("bash-language-server")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::LspClient::new(bash_lsp.stdin.unwrap(), bash_lsp.stdout.unwrap());

                lsp_client.initialize()?;

                Box::new(lsp_client) as Box<dyn LspClient>
            },
            _ => {
                self.channels.0.send(ControllerMessage::NoClient).unwrap();
            }
        };

        let (tx, rx) = std::sync::mpsc::channel();

        self.server_channels.insert(lang.as_ref().to_string(), tx);

        self.clients.insert(lang.as_ref().to_string(), client);

        self.channels.0.send(ControllerMessage::ClientCreated(rx)).unwrap();

        Ok(())
    }

    


}
