
use std::sync::Arc;

use futures::{lock::Mutex, executor::block_on};
use tokio::{io::{BufReader, AsyncBufReadExt, AsyncWriteExt, AsyncReadExt, BufWriter, self}, process::{ChildStdout, ChildStdin}};



pub async fn process_messages(output: Arc<Mutex<BufReader<ChildStdout>>> ) -> io::Result<serde_json::Value> {
    let mut output = output.lock().await;
    let mut header = String::new();
    let mut content_length = 0;
    let mut content_type = String::new();
    while let Ok(bytes_read) = output.read_line(&mut header).await {
        if bytes_read == 0 {
            break;
        }
        if header.starts_with("Content-Length: ") {
            content_length = header[16..].trim().parse::<usize>().expect("Failed to parse content length");
            header.clear();
        }
        if header.starts_with("Content-Type: ") {
            content_type = header[14..].trim().to_string();
            header.clear();
        }
        if header == "\r\n" {
            break;
        }
    }

    let mut body = vec![0; content_length];
    output.read_exact(&mut body).await?;


    let body = match content_type {
        _ => {
            String::from_utf8(body).expect("Failed to parse body as utf8")
        },
    };


    serde_json::from_str(&body).and_then(|json_data| {
        Ok(json_data)
    }).map_err(|err| {
        io::Error::new(io::ErrorKind::Other, err)
    })
}

/*pub trait LspClient {
    //fn process_messages(&mut self) -> io::Result<serde_json::Value>;

    fn get_output(&self) -> Arc<Mutex<BufReader<ChildStdout>>> ;
    fn send_message(&mut self, message: serde_json::Value) -> io::Result<()>;

    fn initialize(&mut self) -> io::Result<()>;
    fn figure_out_capabilities(&mut self) -> io::Result<()>;


    fn send_did_open(&mut self, language_id: &str, uri: &str, text: &str) -> io::Result<()>;

    fn did_change_text(&mut self, uri: &str, version: usize, text: &str) -> io::Result<()>;

    fn did_save_text(&mut self, uri: &str, text: &str) -> io::Result<()>;

    fn will_save_text(&mut self, uri: &str, reason: usize) -> io::Result<()>;

    fn did_close(&mut self, uri: &str) -> io::Result<()>;

    fn send_shutdown(&mut self) -> io::Result<()>;

    fn send_exit(&mut self) -> io::Result<()>;
    
}*/

unsafe impl Send for Client {}

pub struct Client {
    input: BufWriter<ChildStdin>,
    output: BufReader<ChildStdout>,
}

impl Client {
    pub fn new(input: ChildStdin, output: ChildStdout) -> Self {
        eprintln!("Creating client");
        let input = BufWriter::new(input);
        let output = BufReader::new(output);

        Client {
            input,
            output,
        }

    }
}


impl Drop for Client {
    fn drop(&mut self) {
        eprintln!("Dropping client");
        self.send_shutdown().expect("Failed to send shutdown");
        self.send_exit().expect("Failed to send exit");
    }
}

impl Client {

    pub async fn process_messages(&mut self) -> io::Result<serde_json::Value> {
        let mut header = String::new();
        let mut content_length = 0;
        let mut content_type = String::new();
        while let Ok(bytes_read) = self.output.read_line(&mut header).await {
            if bytes_read == 0 {
                break;
            }
            if header.starts_with("Content-Length: ") {
                content_length = header[16..].trim().parse::<usize>().expect("Failed to parse content length");
                header.clear();
            }
            if header.starts_with("Content-Type: ") {
                content_type = header[14..].trim().to_string();
                header.clear();
            }
            if header == "\r\n" {
                break;
            }
        }

        let mut body = vec![0; content_length];
        self.output.read_exact(&mut body).await?;


        let body = match content_type {
            _ => {
                String::from_utf8(body).expect("Failed to parse body as utf8")
            },
        };


        serde_json::from_str(&body).and_then(|json_data| {
            Ok(json_data)
        }).map_err(|err| {
            io::Error::new(io::ErrorKind::Other, err)
        })
    }

    pub async fn figure_out_capabilities(&mut self) -> io::Result<()> {
        self.process_messages().await?;
        Ok(())
    }


    pub fn send_message(&mut self, message: serde_json::Value) -> io::Result<()> {
        eprintln!("Sending messag");
        let future = async {
            let message = serde_json::to_string(&message).expect("Failed to serialize json");
            let message = format!("Content-Length: {}\r\n\r\n{}", message.len(), message);
            self.input.write_all(message.as_bytes()).await.expect("Failed to write");
            self.input.flush().await.expect("Failed to flush");
        };
        block_on(future);
        Ok(())
    }
    pub fn initialize(&mut self) -> io::Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "ClientInfo": {
                    "name": "vi",
                    "version": "0.0.1",
                },
            },
        });
        self.send_message(message)?;
        Ok(())
    }

    pub fn send_did_open(&mut self, language_id: &str, uri: &str, text: &str) -> io::Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": text,
                },
            },
        });
        self.send_message(message)?;
        Ok(())
    }

    pub fn did_change_text(&mut self, uri: &str, version: usize, text: &str) -> io::Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "version": version,
                },
                "contentChanges": [
                    {
                        "text": text,
                    },
                ],
            },
        });
        self.send_message(message)?;
        Ok(())
    }

    pub fn did_save_text(&mut self, uri: &str, text: &str) -> io::Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didSave",
            "params": {
                "textDocument": {
                    "uri": uri,
                },
                "text": text,
            },
        });
        self.send_message(message)?;
        Ok(())
    }

    pub fn will_save_text(&mut self, uri: &str, reason: usize) -> io::Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/willSave",
            "params": {
                "textDocument": {
                    "uri": uri,
                },
                "reason": reason,
            },
        });
        self.send_message(message)?;
        Ok(())
    }

    pub fn did_close(&mut self, uri: &str) -> io::Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didClose",
            "params": {
                "textDocument": {
                    "uri": uri,
                },
            },
        });
        self.send_message(message)?;
        Ok(())
    }

    pub fn send_shutdown(&mut self) -> io::Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "shutdown",
        });
        self.send_message(message)?;
        Ok(())
    }

    pub fn send_exit(&mut self) -> io::Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "exit",
        });
        self.send_message(message)?;
        Ok(())
    }
}

