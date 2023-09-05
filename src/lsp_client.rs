use std::io::{BufReader, BufWriter, self, BufRead, Read, Write};





pub struct LspClient<W: Write,R: Read> {
    input: BufWriter<W>,
    output: BufReader<R>,
    json_data: serde_json::Value,
}

impl<R: Read, W: Write> LspClient<W, R> {
    pub fn new(input: W, output: R) -> Self {
        let input = BufWriter::new(input);
        let output = BufReader::new(output);

        LspClient {
            input,
            output,
            json_data: serde_json::Value::Null,
        }

    }

    pub fn process_messages(&mut self) -> io::Result<()> {

        let mut header = String::new();
        let mut content_length = 0;
        let mut content_type = String::new();
        while let Ok(bytes_read) = self.output.read_line(&mut header) {
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

        
        let json_data: serde_json::Value = serde_json::from_str(&body).expect("Failed to parse json");

        self.json_data = json_data;

        eprintln!("Received message: {:#?}", self.json_data);
        
        Ok(())
    }

    pub fn send_message(&mut self, message: serde_json::Value) -> io::Result<()> {
        let message = serde_json::to_string(&message).expect("Failed to serialize json");
        let message = format!("Content-Length: {}\r\n\r\n{}", message.len(), message);
        self.input.write_all(message.as_bytes())?;
        self.input.flush()?;
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
}


