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
    pub editor_settings: EditorSettings,
    pub mode_keybindings: HashMap<Mode, HashMap<Keys, Command>>,
    
}

impl Default for Settings {
    fn default() -> Self {
        let editor_settings = EditorSettings::default();
        let mut mode_keybindings = HashMap::new();

        let mut normal_keybindings = HashMap::new();

        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('l'),
            modifier: KeyModifiers::NONE,
        }], "right".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char(' '),
            modifier: KeyModifiers::NONE,
        }], "right".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('h'),
            modifier: KeyModifiers::NONE,
        }], "left".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Backspace,
            modifier: KeyModifiers::NONE,
        }], "left".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('j'),
            modifier: KeyModifiers::NONE,
        }], "down".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Enter,
            modifier: KeyModifiers::NONE,
        }], "down".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('k'),
            modifier: KeyModifiers::NONE,
        }], "up".to_string());

        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('0'),
            modifier: KeyModifiers::NONE,
        }], "line_start".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('$'),
            modifier: KeyModifiers::NONE,
        }], "line_end".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::NONE,
        }], "word_start_forward".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('W'),
            modifier: KeyModifiers::NONE,
        }], "word_start_backward".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('B'),
            modifier: KeyModifiers::NONE,
        }], "word_end_forward".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('b'),
            modifier: KeyModifiers::NONE,
        }], "word_end_backward".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('g'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('g'),
            modifier: KeyModifiers::NONE,
        }], "file_top".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Home,
            modifier: KeyModifiers::NONE,
        }], "file_top".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('G'),
            modifier: KeyModifiers::NONE,
        }], "file_bottom".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::End,
            modifier: KeyModifiers::NONE,
        }], "file_bottom".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('b'),
            modifier: KeyModifiers::CONTROL,
        }], "page_up".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('f'),
            modifier: KeyModifiers::CONTROL,
        }], "page_down".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::PageUp,
            modifier: KeyModifiers::NONE,
        }], "page_up".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::PageDown,
            modifier: KeyModifiers::NONE,
        }], "page_down".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('i'),
            modifier: KeyModifiers::NONE,
        }], "insert_before".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('a'),
            modifier: KeyModifiers::NONE,
        }], "insert_after".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('I'),
            modifier: KeyModifiers::NONE,
        }], "insert_before".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('a'),
            modifier: KeyModifiers::NONE,
        }], "insert_after".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('o'),
            modifier: KeyModifiers::NONE,
        }], "insert_bellow".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('O'),
            modifier: KeyModifiers::NONE,
        }], "insert_above".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('r'),
            modifier: KeyModifiers::NONE,
        }], "replace".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('u'),
            modifier: KeyModifiers::NONE,
        }], "undo".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('r'),
            modifier: KeyModifiers::CONTROL,
        }], "redo".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('x'),
            modifier: KeyModifiers::NONE,
        }], "delete_char".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('d'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::NONE,
        }], "delete_word".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('d'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('d'),
            modifier: KeyModifiers::NONE,
        }], "delete_line".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('D'),
            modifier: KeyModifiers::NONE,
        }], "delete_line_remainder".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('y'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('y'),
            modifier: KeyModifiers::NONE,
        }], "copy_line".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('p'),
            modifier: KeyModifiers::NONE,
        }], "paste_after".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('P'),
            modifier: KeyModifiers::NONE,
        }], "paste_before".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char(':'),
            modifier: KeyModifiers::NONE,
        }], "start_command".to_string());

        let mut insert_keybindings = HashMap::new();

        insert_keybindings.insert(vec![Key {
            key: KeyCode::Esc,
            modifier: KeyModifiers::NONE,
        }], "leave".to_string());
        insert_keybindings.insert(vec![Key {
            key: KeyCode::Left,
            modifier: KeyModifiers::NONE,
        }], "left".to_string());
        insert_keybindings.insert(vec![Key {
            key: KeyCode::Right,
            modifier: KeyModifiers::NONE,
        }], "right".to_string());
        insert_keybindings.insert(vec![Key {
            key: KeyCode::Up,
            modifier: KeyModifiers::NONE,
        }], "up".to_string());
        insert_keybindings.insert(vec![Key {
            key: KeyCode::Down,
            modifier: KeyModifiers::NONE,
        }], "down".to_string());
        insert_keybindings.insert(vec![Key {
            key: KeyCode::Home,
            modifier: KeyModifiers::NONE,
        }], "file_top".to_string());
        insert_keybindings.insert(vec![Key {
            key: KeyCode::End,
            modifier: KeyModifiers::NONE,
        }], "file_bottom".to_string());
        insert_keybindings.insert(vec![Key {
            key: KeyCode::Char('b'),
            modifier: KeyModifiers::CONTROL,
        }], "page_up".to_string());
        insert_keybindings.insert(vec![Key {
            key: KeyCode::PageUp,
            modifier: KeyModifiers::NONE,
        }], "page_up".to_string());
        insert_keybindings.insert(vec![Key {
            key: KeyCode::Char('f'),
            modifier: KeyModifiers::CONTROL,
        }], "page_down".to_string());
        insert_keybindings.insert(vec![Key {
            key: KeyCode::PageDown,
            modifier: KeyModifiers::NONE,
        }], "page_down".to_string());

        let mut command_keybindings = HashMap::new();

        command_keybindings.insert(vec![Key {
            key: KeyCode::Esc,
            modifier: KeyModifiers::NONE,
        }], "leave".to_string());
        command_keybindings.insert(vec![Key {
            key: KeyCode::Left,
            modifier: KeyModifiers::NONE,
        }], "left".to_string());
        command_keybindings.insert(vec![Key {
            key: KeyCode::Right,
            modifier: KeyModifiers::NONE,
        }], "right".to_string());
        command_keybindings.insert(vec![Key {
            key: KeyCode::Up,
            modifier: KeyModifiers::NONE,
        }], "start".to_string());
        command_keybindings.insert(vec![Key {
            key: KeyCode::Down,
            modifier: KeyModifiers::NONE,
        }], "end".to_string());

        mode_keybindings.insert("Normal".to_string(), normal_keybindings);
        mode_keybindings.insert("Insert".to_string(), insert_keybindings);
        mode_keybindings.insert("Command".to_string(), command_keybindings);

        
        Self {
            editor_settings,
            mode_keybindings,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct EditorSettings {
    pub line_number: bool,
    pub relative_line_number: bool,
    pub tab_size: usize,
    pub use_spaces: bool,
    pub key_press_duration: usize,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            line_number: true,
            relative_line_number: false,
            tab_size: 4,
            use_spaces: true,
            key_press_duration: 100,
        }
    }
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
