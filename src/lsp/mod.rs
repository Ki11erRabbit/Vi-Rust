use std::{collections::HashMap, sync::{mpsc::{Sender, Receiver}, Arc}, io, process::Stdio, fmt::Display, };
use futures::executor::block_on;
use futures::FutureExt;
use serde_json::Value;
use tokio::process::Command;

use crate::lsp::lsp_utils::{process_json, LSPMessage};

use self::{lsp_client::Client, lsp_utils::{Diagnostics, CompletionList}};

pub mod lsp_client;
pub mod lsp_utils;


unsafe impl Send for LspRequest {}
pub enum LspRequest {
    /// Tells the server to shutdown
    Shutdown,
    /// Tells the server to exit
    Exit,
    /// Requires a URI
    RequestDiagnostic(Box<str>),
    /// Requires a URI, a position, and a way a completion was triggered
    RequestCompletion(Box<str>, (usize, usize), Box<str>),

}

unsafe impl Send for LspResponse {}

pub enum LspResponse {
    PublishDiagnostics(Diagnostics),
    Completion(CompletionList),

}

unsafe impl Send for LspNotification {}
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


unsafe impl Send for ControllerMessage {}

pub enum ControllerMessage {
    /// String is the language id
    Request(Box<str>, LspRequest),
    /// Response to a request
    Response(LspResponse),
    /// Box<str> is the language id
    Notification(Box<str>, LspNotification),
    /// String is the language id
    CreateClient(Box<str>),
    /// Notification to tell the caller how to recieve responses
    /// The receiver is for the language server side
    ClientCreated(Arc<Receiver<ControllerMessage>>),
    /// Notification to tell the caller that there is no client for the language
    NoClient,
    Resend(Box<str>, LspResponse),
    Exit,

    

}


impl Drop for LspController {
    fn drop(&mut self) {
        eprintln!("Dropping lsp controller");
        for (_, client) in self.clients.iter_mut() {
            drop(client);
        }
    }
}

unsafe impl Send for LspController {}

pub struct LspController {
    clients: HashMap<String, Client>,
    //channels: (Sender<ControllerMessage>, Receiver<ControllerMessage>),
    listen: Option<Receiver<ControllerMessage>>,
    response: Option<Sender<ControllerMessage>>,
    server_channels: HashMap<String, (Sender<ControllerMessage>, Arc<Receiver<ControllerMessage>>)>,
    exit: bool,
}



impl LspController {

    pub fn new() -> Self {
        LspController {
            clients: HashMap::new(),
            //channels: std::sync::mpsc::channel(),
            listen: None,
            response: None,
            server_channels: HashMap::new(),
            exit: false,
            
        }
    }

    pub fn set_listen(&mut self, listen: Receiver<ControllerMessage>) {
        self.listen = Some(listen);
    }

    pub fn set_response(&mut self, response: Sender<ControllerMessage>) {
        self.response = Some(response);
    }



    pub fn run(&mut self) -> io::Result<()> {
        eprintln!("Running lsp controller");
        while !self.exit {
            self.check_messages()?;

            
            let future = self.check_clients();
            let _ = block_on(future);
                        

        }
        Ok(())
    }

    async fn check_client(client: &mut Client) -> io::Result<Value> {
        let future = client.process_messages();
        let val = future.await;
        let json = val?;
        Ok(json)
    }

    async fn check_clients(&mut self) -> io::Result<()> {

        for (language, client) in self.clients.iter_mut() {

            let json;
            let mut future = Self::check_client(client).boxed();

            if let Some(value) = (&mut future).now_or_never() {
                json = value?;
                //eprintln!("Got json");
            } else {
                continue;
                //json = future.await?;
            }

            eprintln!("Json for: {} \n{:#?}", language, json);

            match process_json(json).expect("Failed to process json") {
                LSPMessage::Diagnostics(diagnostics) => {
                    //eprintln!("Got diagnostics");
                    let sender = self.server_channels.get(language).unwrap().0.clone();

                    let message = ControllerMessage::Response(
                        LspResponse::PublishDiagnostics(diagnostics)
                    );

                    sender.send(message).expect("Failed to send diagnostics");
                },
                LSPMessage::Completions(completion) => {
                    eprintln!("Got completion");
                    let sender = self.server_channels.get(language).unwrap().0.clone();

                    let message = ControllerMessage::Response(
                        LspResponse::Completion(completion)
                    );

                    sender.send(message).expect("Failed to send completions");
                },
                LSPMessage::None => {
                    //eprintln!("Got none");
                    continue;
                }
            }
        }

        Ok(())
    }


    fn check_messages(&mut self) -> io::Result<()> {
       
        match self.listen.as_ref().unwrap().try_recv() {
            Ok(ControllerMessage::CreateClient(lang)) => {
                self.create_client(lang)
            },
            Ok(ControllerMessage::Request(lang, req)) => {
                self.check_request(lang, req)
            },
            Ok(ControllerMessage::Notification(lang, notif)) => {
                self.check_notification(lang, notif)
            },
            Ok(ControllerMessage::Exit) => {
                self.exit = true;
                return Ok(());
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

    fn check_notification<R>(&mut self, lang: R, notif: LspNotification) -> io::Result<()> where R: AsRef<str> + Display {
        match self.clients.get_mut(&lang.to_string()) {
            Some(client) => {
                match notif {
                    LspNotification::ChangeText(uri, version, text) => {
                        client.did_change_text(uri.as_ref(), version, text.as_ref())?;
                    },
                    LspNotification::Open(uri, text) => {
                        client.send_did_open(&lang.to_string(),uri.as_ref(), text.as_ref())?;
                    },
                    LspNotification::Close(uri) => {
                        client.did_close(uri.as_ref())?;
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
                    LspRequest::RequestDiagnostic(uri) => {
                        client.request_diagnostic(uri.as_ref())?;
                    },
                    LspRequest::RequestCompletion(uri, pos, trigger) => {
                        let trigger = match trigger.as_ref() {
                            "invoked" => 1,
                            "triggerCharacter" => 2,
                            "triggerForIncompleteCompletions" => 3,
                            _ => {
                                return Err(io::Error::new(io::ErrorKind::Other, "Invalid trigger"));
                            }
                        };

                        client.request_completion(uri, pos, trigger)?;
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
        let client = match lang.as_ref() {
            "rust" => {
                if let Some((_, recv)) = self.server_channels.get("rust") {
                    self.response.as_ref().unwrap().send(ControllerMessage::ClientCreated(recv.clone())).unwrap();
                    return Ok(());
                }
                let rust_analyzer = Command::new("rust-analyzer")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::Client::new(rust_analyzer);

                lsp_client.initialize()?;

                lsp_client
            },
            "c" | "cpp" => {

                if let Some((_, recv)) = self.server_channels.get(lang.as_ref()) {
                    self.response.as_ref().unwrap().send(ControllerMessage::ClientCreated(recv.clone())).unwrap();
                    return Ok(());
                }

                let clangd = Command::new("clangd")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::Client::new(clangd);

                lsp_client.initialize()?;

                lsp_client
            },
            "python" => {
                if let Some((_, recv)) = self.server_channels.get(lang.as_ref()) {
                    self.response.as_ref().unwrap().send(ControllerMessage::ClientCreated(recv.clone())).unwrap();
                    return Ok(());
                }
                let python_lsp = Command::new("python-lsp-server")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::Client::new(python_lsp);

                lsp_client.initialize()?;

                lsp_client
            },
            "swift" => {
                if let Some((_, recv)) = self.server_channels.get(lang.as_ref()) {
                    self.response.as_ref().unwrap().send(ControllerMessage::ClientCreated(recv.clone())).unwrap();
                    return Ok(());
                }
                let apple_swift = Command::new("sourcekit-lsp")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::Client::new(apple_swift);

                lsp_client.initialize()?;

                lsp_client
            },
            "go" => {
                if let Some((_, recv)) = self.server_channels.get(lang.as_ref()) {
                    self.response.as_ref().unwrap().send(ControllerMessage::ClientCreated(recv.clone())).unwrap();
                    return Ok(());
                }
                let gopls = Command::new("gopls")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::Client::new(gopls);

                lsp_client.initialize()?;

                lsp_client
            },
            "bash" => {
                if let Some((_, recv)) = self.server_channels.get(lang.as_ref()) {
                    self.response.as_ref().unwrap().send(ControllerMessage::ClientCreated(recv.clone())).unwrap();
                    return Ok(());
                }
                let bash_lsp = Command::new("bash-language-server")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::Client::new(bash_lsp);

                lsp_client.initialize()?;

                lsp_client
            },
            _ => {
                self.response.as_ref().unwrap().send(ControllerMessage::NoClient).unwrap();
                return Ok(());
            }
        };

        let (tx, rx) = std::sync::mpsc::channel();

        let rx = Arc::new(rx);

        self.server_channels.insert(lang.as_ref().to_string(), (tx, rx.clone()));

        //let client = Arc::new(Mutex::new(client));

        self.clients.insert(lang.as_ref().to_string(), client);

        self.response.as_ref().unwrap().send(ControllerMessage::ClientCreated(rx)).unwrap();

        Ok(())
    }

    


}
