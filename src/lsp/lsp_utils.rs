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
    pub version: Option<usize>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Diagnostics {
            diagnostics: Vec::new(),
            uri: String::new(),
            version: Some(0),
        }
    }
    pub fn diagnostics_on_line(&self, line: usize) -> Vec<&Diagnostic> {
        let mut result = Vec::new();
        for diagnostic in &self.diagnostics {
            let start_line = diagnostic.range.start.line;
            let end_line = diagnostic.range.end.line;
            if line >= start_line && line <= end_line {
                result.push(diagnostic);
            }
        }
        result
    }

    pub fn get_diagnostic(&self, line: usize, character: usize) -> Option<&Diagnostic> {
        //eprintln!("{:?}", self.diagnostics);
        for diagnostic in &self.diagnostics {
            let start_line = diagnostic.range.start.line;
            let end_line = diagnostic.range.end.line;
            let start_character = diagnostic.range.start.character;
            let end_character = diagnostic.range.end.character;
            if line >= start_line && line <= end_line {
                if character >= start_character && character <= end_character {
                    return Some(diagnostic);
                }
            }
        }
        None
    }

    pub fn merge(&mut self, other: Diagnostics) {
        self.diagnostics.extend(other.diagnostics);
    }
}

#[derive(Debug, PartialEq, Deserialize, Hash, Eq, Clone)]
pub struct CodeDescription {
    pub href: String,
}

#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone)]
pub struct Data {
    pub rendered: String,
}

#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone)]
pub struct Diagnostic {
    /// The type of the diagnostic.
    pub code: Option<String>,
    /// The description of the diagnostic.
    pub code_description: Option<CodeDescription>,
    /// Additional metadata about the diagnostic.
    pub data: Option<Data>,
    /// The message to display to the user.
    pub message: String,
    /// The range where the error/warning is located in the source code.
    pub range: LSPRange,
    /// The severity of the error/warning.
    pub severity: usize,
    /// The source of the error/warning which is the LSP
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone, Copy)]
pub struct LSPRange {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone, Copy)]
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
            eprintln!("diagnostics");

            let diagnostics: Diagnostics = match serde_json::from_value(obj) {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    return Ok(LSPMessage::None);
                }
            };
            Ok(LSPMessage::Diagnostics(diagnostics))
        }
        _ => {
            println!("Unknown method: {}", method);
            Ok(LSPMessage::None)
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
