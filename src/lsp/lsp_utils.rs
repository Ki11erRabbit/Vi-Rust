use std::io;

use serde::Deserialize;
use serde_json::Value;

use crate::mode::{Promptable, PromptType};


#[derive(Debug, PartialEq)]
pub enum LSPMessage {
    None,
    Diagnostics(Diagnostics),
    Completions(CompletionList),
    Location(LocationResponse),
    
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

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct CompletionList {
    pub isIncomplete: bool,
    pub items: Vec<CompletionItem>,
}

impl CompletionList {
    pub fn generate_text(&self) -> Vec<(String, String)> {
        let mut result = Vec::new();
        for item in &self.items {
            let (insert_text, info) = item.generate_text();
            result.push((insert_text, info));
        }
        result
    }

    pub fn get_completion(&self, index: usize) -> Option<&CompletionItem> {
        self.items.get(index)
    }

    /*pub fn generate_buttons(&self) -> Vec<Box<dyn FnOnce(&dyn Promptable) -> String>> {
        let mut result = Vec::new();
        for item in &self.items {
            let button = item.generate_button();
            result.push(button);
        }
        result
    }*/

    pub fn generate_buttons(&self, max_len: usize) -> PromptType {
        let mut buttons = Vec::new();
        for (i, item) in self.items.iter().enumerate() {
            let (text, info) = item.generate_text();

            let other_half = max_len - text.chars().count() - 1;
            
            let label = format!("{} {:>remain$}", text, info, remain=other_half);

            buttons.push((label, item.generate_button(i)));
        }

        PromptType::Button(buttons, 0)
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, PartialEq,   Clone)]
pub struct CompletionItem {
    pub label: String,
    pub labelDetails: Option<CompletionItemLabelDetails>,
    pub kind: usize,
    pub score: Option<f64>,
    pub tags: Option<Vec<usize>>,
    pub detail: Option<String>,
    pub documentation: Option<Value>,
    pub deprecated: Option<bool>,
    pub preselect: Option<bool>,
    pub sortText: Option<String>,
    pub filterText: Option<String>,
    pub insertText: Option<String>,
    pub insertTextFormat: Option<usize>,
    pub insertTextMode: Option<usize>,
    pub textEdit: Option<Value>,
    pub textEditText: Option<String>,
    pub additionalTextEdits: Option<Vec<TextEdit>>,
    pub commitCharacters: Option<Vec<String>>,
    pub command: Option<Command>,
    pub data: Option<Value>,
}

impl CompletionItem {
    pub fn generate_text(&self) -> (String, String) {

        let kind = match self.kind {
            1 => "Text",
            2 => "Method",
            3 => "Function",
            4 => "Constructor",
            5 => "Field",
            6 => "Variable",
            7 => "Class",
            8 => "Interface",
            9 => "Module",
            10 => "Property",
            11 => "Unit",
            12 => "Value",
            13 => "Enum",
            14 => "Keyword",
            15 => "Snippet",
            16 => "Color",
            17 => "File",
            18 => "Reference",
            19 => "Folder",
            20 => "EnumMember",
            21 => "Constant",
            22 => "Struct",
            23 => "Event",
            24 => "Operator",
            25 => "TypeParameter",
            _ => "Unknown",
        };
        
        let info = format!("({}) {}", kind, self.label);


        let mut insert_text = String::new();
        if let Some(text_edit) = &self.textEdit {
            let text_edit: TextEdit = serde_json::from_value(text_edit.clone()).unwrap();
            insert_text = text_edit.newText.clone();
        } else if let Some(text_edit) = &self.textEditText {
            insert_text = text_edit.clone();
        } else if let Some(text_edit) = &self.insertText {
            insert_text = text_edit.clone();
        }
        (insert_text, info)
    }

    pub fn generate_button(&self, index: usize) -> Box<dyn Fn(&dyn Promptable) -> String> {
        /*let mut insert_text = String::new();
        if let Some(text_edit) = &self.textEdit {
            let text_edit: TextEdit = serde_json::from_value(text_edit.clone()).unwrap();
            insert_text = text_edit.newText.clone();
        } else if let Some(text_edit) = &self.textEditText {
            insert_text = text_edit.clone();
        } else if let Some(text_edit) = &self.insertText {
            insert_text = text_edit.clone();
        }*/
        Box::new(move |_| {
            //insert_text
            index.to_string()
        })
    }

    pub fn get_edit_text(&self) -> Option<TextEditType> {
        if let Some(text_edit) = &self.textEdit {
            if let Ok(edit_text) = serde_json::from_value::<TextEdit>(text_edit.clone()) {
                return Some(TextEditType::TextEdit(edit_text));
            }
            else if let Ok(edit_text) = serde_json::from_value::<InsertReplaceEdit>(text_edit.clone()) {
                return Some(TextEditType::InsertReplaceEdit(edit_text));
            }
            else {
                return None;
            }
        }
        None
    }
}

#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone)]
pub struct CompletionItemLabelDetails {
    detail: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone)]
pub enum DocumentationType {
    String(String),
    MarkupContent(MarkupContent),
}

#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone)]
pub struct MarkupContent {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone)]
pub enum TextEditType {
    TextEdit(TextEdit),
    InsertReplaceEdit(InsertReplaceEdit),
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone)]
pub struct TextEdit {
    pub range: LSPRange,
    pub newText: String,
}

impl TextEdit {
    pub fn get_range(&self) -> ((usize, usize), (usize, usize)) {
        let start = self.range.start;
        let end = self.range.end;
        ((start.character, start.line), (end.character, end.line))
    }
}

#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone)]
pub struct InsertReplaceEdit {
    pub insert: LSPRange,
    pub replace: LSPRange,
    pub new_text: String,
}

#[derive(Debug, Deserialize, PartialEq,  Eq, Clone)]
pub struct Command {
    pub title: String,
    pub command: String,
    pub arguments: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone)]
pub enum LocationResponse {
    Location(Location),
    LocationLink(LocationLink),
    Locations(Vec<Location>),
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone)]
pub struct Location {
    pub uri: String,
    pub range: LSPRange,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone)]
pub struct LocationLink {
    pub originSelectionRange: Option<LSPRange>,
    pub targetUri: String,
    pub targetRange: LSPRange,
    pub targetSelectionRange: LSPRange,
}

pub fn process_json(json: Value) -> io::Result<LSPMessage> {


    if json["method"] != Value::Null {

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
            },

            _ => {
                println!("Unknown method: {}", method);
                Ok(LSPMessage::None)
            }
        }
    }
    else if json["id"] != Value::Null {
        let id: usize = match serde_json::from_value(json["id"].clone()) {
            Ok(value) => value,
            Err(e) => {
                eprintln!("Id Error: {:?}", e);
                return Ok(LSPMessage::None);
            }
        };
        match id {
            2 => {
                let obj = json["result"].clone();
                eprintln!("completion");

                let completion_list: CompletionList = match serde_json::from_value(obj) {
                    Ok(value) => value,
                    Err(e) => {
                        eprintln!("Completion Error: {:?}", e);
                        return Ok(LSPMessage::None);
                    }
                };
                Ok(LSPMessage::Completions(completion_list))
            },
            3 => {
                let obj = json["result"].clone();

                if obj.is_array() {
                    let locations: Vec<Location> = match serde_json::from_value(obj) {
                        Ok(value) => value,
                        Err(e) => {
                            eprintln!("Location Error: {:?}", e);
                            return Ok(LSPMessage::None);
                        }
                    };

                    let locations = LocationResponse::Locations(locations);

                    Ok(LSPMessage::Location(locations))
                }
                else if obj.is_object() {
                    if json.get("uri").is_some() {
                        let location: Location = match serde_json::from_value(obj) {
                            Ok(value) => value,
                            Err(e) => {
                                eprintln!("Location Error: {:?}", e);
                                return Ok(LSPMessage::None);
                            }
                        };

                        let location = LocationResponse::Location(location);

                        Ok(LSPMessage::Location(location))
                    }
                    else {
                        let location_link: LocationLink = match serde_json::from_value(obj) {
                            Ok(value) => value,
                            Err(e) => {
                                eprintln!("Location Error: {:?}", e);
                                return Ok(LSPMessage::None);
                            }
                        };

                        let location = LocationResponse::LocationLink(location_link);

                        Ok(LSPMessage::Location(location))
                    }

                }
                else {
                    Ok(LSPMessage::None)
                }
                
                
            },
            _ => {
                eprintln!("Unknown id: {}", id);
                Ok(LSPMessage::None)
            }
        }
        
    }
    else {
        eprintln!("Error: no method or result");
        Ok(LSPMessage::None)
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
