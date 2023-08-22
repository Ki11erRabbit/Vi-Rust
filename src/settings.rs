use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};
use serde::Deserialize;


#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Key {
    pub key: KeyCode,
    pub modifier: KeyModifiers,
}

pub type Mode = String;
pub type Command = String;

pub type Keys = Vec<Key>;

#[derive(Debug)]
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
    key_press_duration: usize,
}

fn parse_key(value: &toml::Value) -> Keys {
    match value {
        toml::Value::String(string) => {
            if string.len() == 1 {
                let key = string.chars().next().unwrap();
                vec![Key {
                    key: KeyCode::Char(key),
                    modifier: KeyModifiers::NONE,
                }]
            } else {
                let key = match string.as_str() {
                    "space" => KeyCode::Char(' '),
                    "backspace" => KeyCode::Backspace,
                    "enter" => KeyCode::Enter,
                    "left" => KeyCode::Left,
                    "right" => KeyCode::Right,
                    "up" => KeyCode::Up,
                    "down" => KeyCode::Down,
                    "home" => KeyCode::Home,
                    "end" => KeyCode::End,
                    "page-up" => KeyCode::PageUp,
                    "page-down" => KeyCode::PageDown,
                    "tab" => KeyCode::Tab,
                    "back-tab" => KeyCode::BackTab,
                    "delete" => KeyCode::Delete,
                    "insert" => KeyCode::Insert,
                    "esc" => KeyCode::Esc,
                    "caps-lock" => KeyCode::CapsLock,
                    "scroll-lock" => KeyCode::ScrollLock,
                    "num-lock" => KeyCode::NumLock,
                    "print-screen" => KeyCode::PrintScreen,
                    "pause" => KeyCode::Pause,
                    "menu" => KeyCode::Menu,
                    "keypad-begin" => KeyCode::KeypadBegin,
                    //todo: add function keys
                    x => {
                        println!("unimplemented key: {}", x);
                        unimplemented!()},
                };

                vec![Key {
                    key,
                    modifier: KeyModifiers::NONE,
                }]
            }
        },
        toml::Value::Table(table) => {

            if let Some(key) = table.get("key") {
                let key = parse_key(key);
                let mod_keys = table["mod"].as_array().expect("modifier keys were not an array").iter().map(|value| {
                    match value.as_str().unwrap() {
                        "ctrl" => KeyModifiers::CONTROL,
                        "alt" => KeyModifiers::ALT,
                        "shift" => KeyModifiers::SHIFT,
                        _ => unimplemented!(),
                    }
                }).fold(KeyModifiers::NONE, |acc, modifier| {
                    acc | modifier
                });

                vec![Key {
                    key: key[0].key,
                    modifier: mod_keys,
                }]
            }
            else if let Some(keys) = table.get("keys") {
                let keys = keys.as_array().expect("keys were not an array").iter().map(|value| {
                    parse_key(value).remove(0)
                }).collect();

                keys
            }
            else {
                unreachable!()
            }

        },
        _ => unreachable!(),
    }
}

fn parse_keys(value: &toml::Value) -> Vec<Keys> {
    match value {
        toml::Value::String(_) => {
            let key = parse_key(value);
            vec![key]
        },
        toml::Value::Table(_) => {
            let key = parse_key(value);
            vec![key]
        },
        toml::Value::Array(array) => {
            array.iter().map(|value| {
                parse_key(value)
            }).collect()
        },
        _ => unreachable!(),
    }
}

fn parse_keybindings(table: &toml::Value, commands: &[String]) -> HashMap<Keys, Command> {
    let mut keybindings = HashMap::new();

    for command in commands.iter() {
        if let Some(value) = table.get(command) {
            let keys = parse_keys(value);

            for key in keys {
                keybindings.insert(key, command.to_string());
            }
        }
    }

    keybindings
}


pub fn read_settings(settings_file: &str, mode_info: HashMap<String,Vec<String>>) -> Settings {
    println!("settings file: \n{}", settings_file);
    let table = settings_file.parse::<toml::Table>().unwrap();

    println!("table: {}", table["editor"].to_string());

    let mut editor_string = table["editor"].to_string();
    editor_string.remove(0);
    editor_string.pop();

    let editor_string = editor_string.replace(",", "\n");

    let editor_settings = toml::from_str(&editor_string).unwrap();

    let mut mode_keybindings = HashMap::new();
    for name in mode_info.keys() {
        let keybindings = parse_keybindings(&table[name], mode_info.get(name).unwrap());

        mode_keybindings.insert(name.to_string(), keybindings);
    }
    
    Settings {
        editor_settings,
        mode_keybindings,
    }
}


#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_config() {
        let settings = fs::read_to_string("config.toml").unwrap();

        let normal_settings = vec![
            "left".to_string(),
            "right".to_string(),
            "up".to_string(),
            "down".to_string(),
            "line_start".to_string(),
            "line_end".to_string(),
            "word_start_forward".to_string(),
            "word_start_backward".to_string(),
            "word_end_forward".to_string(),
            "word_end_backward".to_string(),
            "file_top".to_string(),
            "file_bottom".to_string(),
            "page_up".to_string(),
            "page_down".to_string(),
            "insert_before".to_string(),
            "start_command".to_string()];

        let insert_settings = vec![
            "leave".to_string(),
            "left".to_string(),
            "right".to_string(),
            "up".to_string(),
            "down".to_string(),
            "file_top".to_string(),
            "file_bottom".to_string(),
            "page_up".to_string(),
            "page_down".to_string()];

        let command_settings = vec![
            "leave".to_string(),
            "left".to_string(),
            "right".to_string(),
            "start".to_string(),
            "end".to_string()];

        let mut mode_settings = HashMap::new();

        mode_settings.insert("normal".to_string(), normal_settings);
        mode_settings.insert("insert".to_string(), insert_settings);
        mode_settings.insert("command".to_string(), command_settings);
            
        

        let settings = read_settings(&settings, mode_settings);

        println!("{:?}", settings);

    }

}
