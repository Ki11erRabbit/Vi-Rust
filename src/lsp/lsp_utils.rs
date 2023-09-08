use std::io;

use serde::Deserialize;
use serde_json::Value;


#[derive(Debug, PartialEq)]
pub enum LSPMessage {
    None,
    Diagnostics(Diagnostics),
    
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Diagnostics {
    pub diagnostics: Vec<Diagnostic>,
    pub uri: String,
    pub version: usize,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub range: LSPRange,
    pub severity: usize,
    pub source: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct LSPRange {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}




pub fn process_json(json: Value) -> io::Result<LSPMessage> {

    match json["method"] {
        Value::Null => {
            return Ok(LSPMessage::None);
        }
        _ => {}
    }
    
    let method = json["method"].as_str().unwrap();
    match method {
        "textDocument/publishDiagnostics" => {
            let obj = json["params"].clone();
            
            let diagnostics: Diagnostics = serde_json::from_value(obj)?;
            Ok(LSPMessage::Diagnostics(diagnostics))
        }
        _ => {
            println!("Unknown method: {}", method);
            Ok(LSPMessage::Diagnostics(Diagnostics {
                diagnostics: Vec::new(),
                uri: String::new(),
                version: 0,
            }))
        }
    }
    
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json(){
        let json = r#"
        {
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///home/username/project/src/main.rs",
                "version": 0,
                "diagnostics": [
                    {
                        "range": {
                            "start": {
                                "line": 0,
                                "character": 0
                            },
                            "end": {
                                "line": 0,
                                "character": 0
                            }
                        },
                        "severity": 1,
                        "code": "E0001",
                        "source": "rustc",
                        "message": "this is a test"
                    }
                ]
            }
        }
        "#;
        let value = process_json(serde_json::from_str(json).unwrap());


        if value.is_err() {
            println!("Error: {:?}", value.err());
            assert!(false);
            return;
        }

        let value = value.unwrap();

        assert_eq!(value, LSPMessage::Diagnostics(Diagnostics {
            diagnostics: vec![
                Diagnostic {
                    code: "E0001".to_string(),
                    message: "this is a test".to_string(),
                    range: LSPRange {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 0,
                        },
                    },
                    severity: 1,
                    source: "rustc".to_string(),
                }
            ],
            uri: "file:///home/username/project/src/main.rs".to_string(),
            version: 0,
        }));
        
    }


}
