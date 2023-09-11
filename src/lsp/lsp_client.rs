

use futures::executor::block_on;
use tokio::{io::{BufReader, AsyncBufReadExt, AsyncWriteExt, AsyncReadExt, BufWriter, self}, process::{ChildStdout, ChildStdin, Child}};




unsafe impl Send for Client {}

pub struct Client {
    child: Child,
    input: BufWriter<ChildStdin>,
    output: BufReader<ChildStdout>,
}

impl Client {
    pub fn new(mut child: Child) -> Self {
        let input = child.stdin.take().expect("Failed to get stdin");
        let output = child.stdout.take().expect("Failed to get stdout");
        let input = BufWriter::new(input);
        let output = BufReader::new(output);

        Client {
            child,
            input,
            output,
        }

    }
}


impl Drop for Client {
    fn drop(&mut self) {
        self.send_shutdown().expect("Failed to send shutdown");
        self.send_exit().expect("Failed to send exit");
        let future = async {
            self.child.wait().await.expect("Failed to wait for child");
        };
        block_on(future);
    }
}

impl Client {

    pub async fn process_messages(&mut self) -> io::Result<serde_json::Value> {

        let value = async {
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

            //eprintln!("Body: {}", body);


            serde_json::from_str(&body).and_then(|json_data| {
                Ok(json_data)
            }).map_err(|err| {
                io::Error::new(io::ErrorKind::Other, err)
            })
        };
        value.await
    }

    pub async fn figure_out_capabilities(&mut self) -> io::Result<()> {
        self.process_messages().await?;
        Ok(())
    }


    pub fn send_message(&mut self, message: serde_json::Value) -> io::Result<()> {
        //eprintln!("Sending messag");
        let future = async {
            let message = serde_json::to_string(&message).expect("Failed to serialize json");
            let message = format!("Content-Length: {}\r\n\r\n{}", message.len(), message);
            self.input.write_all(message.as_bytes()).await.expect("Failed to write");
            match self.input.flush().await {
                Ok(_) => {},
                Err(err) => {
                    eprintln!("Failed to flush: {}", err);
                },
            }
            
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
                "capabilities": {
                    "diagnostics": {
                        "refreshSupport": true,
                    },
                    "textDocument": {
                        "completion": {
                            "snippetSupport": true,
                            "insertReplaceSupport": true,
                            "documentationFormat": [
                                "markdown",
                                "plaintext",
                            ],
                            "completionItemKind": {
                                "valueSet": [
                                    1,
                                    2,
                                    3,
                                    4,
                                    5,
                                    6,
                                    7,
                                    8,
                                    9,
                                    10,
                                    11,
                                    12,
                                    13,
                                    14,
                                    15,
                                    16,
                                    17,
                                    18,
                                    19,
                                    20,
                                    21,
                                    22,
                                    23,
                                    24,
                                    25,
                                ],
                            },
                        },
                    },
                },
            },
        });
        self.send_message(message)?;

        self.send_inialized()?;
        
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

    pub fn request_diagnostic(&mut self, uri: &str) -> io::Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/diagnostic",
            "params": {
                "textDocument": uri,
            },
        });
        self.send_message(message)?;
        Ok(())
    }

    pub fn request_completion(&mut self, uri: Box<str>, (x, y): (usize, usize), trigger: usize) -> io::Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": {
                    "uri": uri,
                },
                "position": {
                    "line": y,
                    "character": x,
                },
                "context": {
                    "triggerKind": trigger,
                },
            },
        });
        self.send_message(message)?;
        Ok(())
    }

    pub fn send_inialized(&mut self) -> io::Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {},
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
            "method": "exit",
        });
        self.send_message(message)?;
        Ok(())
    }
}

