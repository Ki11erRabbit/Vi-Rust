use std::{collections::HashMap, cell::RefCell};
use arboard::Clipboard;


pub trait RegisterUtils<T> {
    /// Get the value of a register.
    fn get(&self, name: T) -> Option<&String>;
    /// Set the value of a register.
    fn set(&mut self, name: T, value: String);
}


pub struct Registers {
    clipboard: Result<RefCell<Clipboard>, Option<String>>,
    named: HashMap<String, String>,
    numbered: HashMap<usize, String>,
}


impl Registers {
    pub fn new() -> Registers {

        let clipboard = match Clipboard::new() {
            Ok(clipboard) => {
                eprintln!("using os clipboard");
                Ok(RefCell::new(clipboard))},
            Err(_) => Err(None),
        };
        
        Registers {
            clipboard,//: ClipboardContext::new().map_err(|_| None).map(RefCell::new),
            named: HashMap::new(),
            numbered: HashMap::new(),
        }
    }

    pub fn get_clipboard(&self) -> Option<String> {
        match &self.clipboard {
            Ok(clipboard) => {
                eprintln!("getting from os clipboard");
                let mut clipboard = clipboard.borrow_mut();
                match clipboard.get_text() {
                    Ok(contents) => Some(contents),
                    Err(_) => None,
                }
            },
            Err(cb) => {
                cb.clone()
            }
        }
    }

    pub fn set_clipboard(&mut self, value: String) {
        match &mut self.clipboard {
            Ok(clipboard) => {
                let mut clipboard = clipboard.borrow_mut();
                match clipboard.set_text(value) {
                    Ok(_) => {},
                    Err(_) => {}
                }
            },
            Err(cb) => {
                cb.replace(value);
            }
        }
    }
    
}

impl RegisterUtils<usize> for Registers {
    fn get(&self, name: usize) -> Option<&String> {
        self.numbered.get(&name)
    }

    fn set(&mut self, name: usize, value: String) {
        self.numbered.insert(name, value);
    }

}

impl RegisterUtils<String> for Registers {
    fn get(&self, name: String) -> Option<&String> {
        self.named.get(&name)
    }

    fn set(&mut self, name: String, value: String) {
        self.named.insert(name, value);
    }
}
