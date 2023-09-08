use std::{collections::HashMap, sync::{mpsc::{Sender, Receiver}, Arc}, io, process::Stdio, fmt::Display, task::Poll};
use futures::{executor::block_on, poll};
use serde_json::Value;
use tokio::process::Command;

use self::lsp_client::{LspClient, process_messages, Client};

pub mod lsp_client;


unsafe impl Send for LspRequest {}
pub enum LspRequest {
    /// Tells the server to shutdown
    Shutdown,
    /// Tells the server to exit
    Exit,

}

unsafe impl Send for LspResponse {}

pub enum LspResponse {

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
    /// Box<str> is the language id
    Response(Box<str>, LspResponse),
    /// Box<str> is the language id
    Notification(Box<str>, LspNotification),
    /// String is the language id
    CreateClient(Box<str>),
    /// Notification to tell the caller how to recieve responses
    /// The receiver is for the language server side
    ClientCreated(Arc<Receiver<ControllerMessage>>),
    /// Notification to tell the caller that there is no client for the language
    NoClient,
    Resend
    

}

unsafe impl Send for LspController {}

pub struct LspController {
    clients: HashMap<String, Client>,
    //channels: (Sender<ControllerMessage>, Receiver<ControllerMessage>),
    listen: Option<Receiver<ControllerMessage>>,
    response: Option<Sender<ControllerMessage>>,
    server_channels: HashMap<String, (Sender<ControllerMessage>, Arc<Receiver<ControllerMessage>>)>,
    runtime: tokio::runtime::Runtime,
    tasks: HashMap<String, tokio::task::JoinHandle<io::Result<Value>>>,
}



impl LspController {

    pub fn new() -> Self {
        LspController {
            clients: HashMap::new(),
            //channels: std::sync::mpsc::channel(),
            listen: None,
            response: None,
            server_channels: HashMap::new(),
            runtime: tokio::runtime::Runtime::new().unwrap(),
            tasks: HashMap::new(),
            
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
        loop {
            self.check_messages()?;


            if self.tasks.len() < self.server_channels.len() {
                self.check_clients();
            }

            eprintln!("Checked clients");
        }
    }

    fn check_clients(&mut self) {
        for (language, client) in self.clients.iter_mut() {

            if self.tasks.contains_key(language) {
                continue;
            }
                
            let output = client.get_output().clone();

            let task = self.runtime.spawn_blocking(move || {
                let json = block_on(process_messages(output));
                json
            });

            self.tasks.insert(language.to_string(), task);


                
            
            //let json = process_messages(output).await.expect("Error processing messages");

            //todo: process json




        }
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
                }
            },
            None => {
                return Err(io::Error::new(io::ErrorKind::Other, "No client for language"));
            }
        }

        Ok(())
    }

    fn create_client<R>(&mut self, lang: R) -> io::Result<()> where R: AsRef<str> {
        let client: Box<dyn LspClient + Send> = match lang.as_ref() {
            "rust" => {
                let rust_analyzer = Command::new("rust-analyzer")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::Client::new(rust_analyzer.stdin.unwrap(), rust_analyzer.stdout.unwrap());

                lsp_client.initialize()?;

                Box::new(lsp_client)
            },
            "c" | "cpp" => {
                let clangd = Command::new("clangd")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::Client::new(clangd.stdin.unwrap(), clangd.stdout.unwrap());

                lsp_client.initialize()?;

                Box::new(lsp_client)
            },
            "python" => {
                let python_lsp = Command::new("python-lsp-server")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::Client::new(python_lsp.stdin.unwrap(), python_lsp.stdout.unwrap());

                lsp_client.initialize()?;

                Box::new(lsp_client)
            },
            "swift" => {
                let apple_swift = Command::new("sourcekit-lsp")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::Client::new(apple_swift.stdin.unwrap(), apple_swift.stdout.unwrap());

                lsp_client.initialize()?;

                Box::new(lsp_client)
            },
            "go" => {
                let gopls = Command::new("gopls")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::Client::new(gopls.stdin.unwrap(), gopls.stdout.unwrap());

                lsp_client.initialize()?;

                Box::new(lsp_client)
            },
            "bash" => {
                let bash_lsp = Command::new("bash-language-server")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()?;

                let mut lsp_client = lsp_client::Client::new(bash_lsp.stdin.unwrap(), bash_lsp.stdout.unwrap());

                lsp_client.initialize()?;

                Box::new(lsp_client)
            },
            _ => {
                self.response.as_ref().unwrap().send(ControllerMessage::NoClient).unwrap();
                return Ok(());
            }
        };

        let (tx, rx) = std::sync::mpsc::channel();

        let rx = Arc::new(rx);

        self.server_channels.insert(lang.as_ref().to_string(), (tx, rx.clone()));

        self.clients.insert(lang.as_ref().to_string(), client);

        self.response.as_ref().unwrap().send(ControllerMessage::ClientCreated(rx)).unwrap();

        Ok(())
    }

    


}
