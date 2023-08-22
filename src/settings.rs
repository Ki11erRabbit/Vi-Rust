use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};
use serde::Deserialize;


#[derive(Debug, Clone, Hash)]
pub struct Key {
    pub key: KeyCode,
    pub modifier: KeyModifiers,
}

pub type Mode = String;
pub type Command = String;

pub type Keys = Vec<Key>;

pub struct Settings {
    editor_settings: EditorSettings,
    mode_keybindings: HashMap<Mode, HashMap<Keys, Command>>,
    
}

#[derive(Debug, Deserialize)]
pub struct EditorSettings {
    line_number: bool,
    relative_line_number: bool,
    tab_size: usize,
    use_spaces: bool,
}




pub fn read_settings(settings_file: &str) -> Settings {
    

}

