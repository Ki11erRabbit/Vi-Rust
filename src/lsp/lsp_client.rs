use async_trait::async_trait;
use std::io::{self, Write, BufWriter};
use tokio::io::{AsyncRead, BufReader, };

#[async_trait]
pub trait LspClient {
    async fn process_messages(&mut self) -> io::Result<serde_json::Value>;
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
    
}

pub struct Client<W: AsyncWrite,R: AsyncRead> {
    input: BufWriter<W>,
    output: BufReader<R>,
}

impl<R: AsyncRead, W: AsyncWrite> Client<W, R> {
    pub fn new(input: W, output: R) -> Self {
        let input = BufWriter::new(input);
        let output = BufReader::new(output);

        Client {
            input,
            output,
        }

    }
}


impl<R: AsyncRead, W: AsyncWrite> Drop for Client<W, R> {
    fn drop(&mut self) {
        self.send_shutdown().expect("Failed to send shutdown");
        self.send_exit().expect("Failed to send exit");
    }
}

#[async_trait]
impl<R: AsyncRead, W: AsyncWrite> LspClient for Client<W, R> {

    async fn process_messages(&mut self) -> io::Result<serde_json::Value> {

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
        self.output.read_exact(&mut body)?;


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

    fn send_message(&mut self, message: serde_json::Value) -> io::Result<()> {
        let message = serde_json::to_string(&message).expect("Failed to serialize json");
        let message = format!("Content-Length: {}\r\n\r\n{}", message.len(), message);
        self.input.write_all(message.as_bytes())?;
        self.input.flush()?;
        Ok(())
    }

    fn initialize(&mut self) -> io::Result<()> {
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

    fn figure_out_capabilities(&mut self) -> io::Result<()> {
        self.process_messages()
    }


    fn send_did_open(&mut self, language_id: &str, uri: &str, text: &str) -> io::Result<()> {
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

    fn did_change_text(&mut self, uri: &str, version: usize, text: &str) -> io::Result<()> {
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

    fn did_save_text(&mut self, uri: &str, text: &str) -> io::Result<()> {
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

    fn will_save_text(&mut self, uri: &str, reason: usize) -> io::Result<()> {
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

    fn did_close(&mut self, uri: &str) -> io::Result<()> {
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

    fn send_shutdown(&mut self) -> io::Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "shutdown",
        });
        self.send_message(message)?;
        Ok(())
    }

    fn send_exit(&mut self) -> io::Result<()> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "exit",
        });
        self.send_message(message)?;
        Ok(())
    }
}

