use core::fmt;
use std::{collections::HashMap, rc::Rc};

use crossterm::{event::{KeyCode, KeyModifiers, KeyEvent}, style::{Attribute, Color}};
use serde::Deserialize;

#[macro_export]
macro_rules! apply_colors {
    ($input:expr, $settings:expr) => {
        {
            let mut inter = $input.with($settings.foreground_color)
                .on($settings.background_color)
                .underline($settings.underline_color);

            for attribute in $settings.attributes.iter() {
                inter = inter.attribute(*attribute);
            }
            inter
        }
    };
}

pub struct KeyCodeWrapper(KeyCode);

impl fmt::Display for KeyCodeWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = match self.0 {
            KeyCode::Char(c) => format!("{}", c),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::Backspace => "Backspace".to_string(),
            KeyCode::Left => "Left".to_string(),
            KeyCode::Right => "Right".to_string(),
            KeyCode::Up => "Up".to_string(),
            KeyCode::Down => "Down".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PageUp".to_string(),
            KeyCode::PageDown => "PageDown".to_string(),
            KeyCode::Delete => "Delete".to_string(),
            KeyCode::Insert => "Insert".to_string(),
            KeyCode::F(u16) => format!("F{}", u16),
            KeyCode::Null => "Null".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::BackTab => "BackTab".to_string(),
            KeyCode::CapsLock => "CapsLock".to_string(),
            KeyCode::NumLock => "NumLock".to_string(),
            KeyCode::ScrollLock => "ScrollLock".to_string(),
            KeyCode::PrintScreen => "PrintScreen".to_string(),
            KeyCode::Pause => "Pause".to_string(),
            KeyCode::Menu => "Menu".to_string(),
            _ => "Unknown".to_string(),
        };
        write!(f, "{}", key)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Key {
    pub key: KeyCode,
    pub modifier: KeyModifiers,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut key = String::new();
        if self.modifier.contains(KeyModifiers::CONTROL) {
            key.push_str("C-");
        }
        if self.modifier.contains(KeyModifiers::ALT) {
            key.push_str("M-");
        }
        if self.modifier.contains(KeyModifiers::SHIFT) {
            key.push_str("S-");
        }
        key.push_str(&format!("{}", KeyCodeWrapper(self.key)));
        write!(f, "{}", key)
    }
}

impl From<KeyEvent> for Key {
    fn from(key_event: KeyEvent) -> Self {

        let modifier = if let KeyCode::Char(c) = key_event.code {
            if c.is_ascii_uppercase() && key_event.modifiers | KeyModifiers::SHIFT == KeyModifiers::SHIFT {
                KeyModifiers::NONE
            } else {
                key_event.modifiers
            }
        } else {
            key_event.modifiers
        };
        
        Self {
            key: key_event.code,
            modifier,
        }
    }
}

pub type Mode = String;
pub type Command = String;

pub type Keys = Vec<Key>;

#[derive(Debug, Clone)]
pub struct Settings {
    pub editor_settings: EditorSettings,
    pub mode_keybindings: HashMap<Mode, HashMap<Keys, Command>>,
    pub colors: EditorColors,
    
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
            key: KeyCode::Right,
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
            key: KeyCode::Left,
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
            key: KeyCode::Down,
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
            key: KeyCode::Up,
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
        }], "insert_beginning".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('A'),
            modifier: KeyModifiers::NONE,
        }], "insert_end".to_string());
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
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('Z'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('Z'),
            modifier: KeyModifiers::NONE,
        }], "q!".to_string());

        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('i'),
            modifier: KeyModifiers::CONTROL,
        }], "jump next".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Tab,
            modifier: KeyModifiers::NONE,
        }], "jump next".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('o'),
            modifier: KeyModifiers::CONTROL,
        }], "jump prev".to_string());

        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::CONTROL,
        }, Key {
            key: KeyCode::Char('s'),
            modifier: KeyModifiers::NONE,
        }], "horizontal_split".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::CONTROL,
        }, Key {
            key: KeyCode::Char('v'),
            modifier: KeyModifiers::NONE,
        }], "vertical_split".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::CONTROL,
        }, Key {
            key: KeyCode::Char('h'),
            modifier: KeyModifiers::NONE,
        }], "pane_left".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::CONTROL,
        }, Key {
            key: KeyCode::Char('j'),
            modifier: KeyModifiers::NONE,
        }], "pane_down".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::CONTROL,
        }, Key {
            key: KeyCode::Char('k'),
            modifier: KeyModifiers::NONE,
        }], "pane_up".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::CONTROL,
        }, Key {
            key: KeyCode::Char('l'),
            modifier: KeyModifiers::NONE,
        }], "pane_right".to_string());

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
        insert_keybindings.insert(vec![Key {
            key: KeyCode::Char('i'),
            modifier: KeyModifiers::CONTROL,
        }], "jump next".to_string());
        insert_keybindings.insert(vec![Key {
            key: KeyCode::Char('o'),
            modifier: KeyModifiers::CONTROL,
        }], "jump prev".to_string());

        insert_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::CONTROL,
        }, Key {
            key: KeyCode::Char('s'),
            modifier: KeyModifiers::NONE,
        }], "horizontal_split".to_string());
        insert_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::CONTROL,
        }, Key {
            key: KeyCode::Char('v'),
            modifier: KeyModifiers::NONE,
        }], "vertical_split".to_string());

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

        let colors = EditorColors::default();
        
        Self {
            editor_settings,
            mode_keybindings,
            colors,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct EditorSettings {
    pub line_number: bool,
    pub relative_line_number: bool,
    pub tab_size: usize,
    pub use_spaces: bool,
    pub key_timeout: u64,
    pub border: bool,
    pub minimum_width: usize,
    pub minimum_height: usize,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            line_number: true,
            relative_line_number: true,
            tab_size: 4,
            use_spaces: true,
            key_timeout: 3000,
            border: true,
            minimum_width: 24,
            minimum_height: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorScheme {
    pub foreground_color: Color,
    pub background_color: Color,
    pub underline_color: Color,
    pub attributes: Rc<Vec<Attribute>>,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            foreground_color: Color::Reset,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }
    }
}

impl ColorScheme {
    pub fn add_attribute(&self, attribute: Attribute) -> Self {
        let mut attributes = *self.attributes.clone();
        attributes.push(attribute);
        Self {
            foreground_color: self.foreground_color,
            background_color: self.background_color,
            underline_color: self.underline_color,
            attributes: Rc::new(attributes),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorColors {
    pub pane: ColorScheme,
    pub ui: ColorScheme,
    pub mode: HashMap<String, ColorScheme>,
}

impl Default for EditorColors {
    fn default() -> Self {
        let mut mode = HashMap::new();

        mode.insert("Normal".to_string(), ColorScheme {
            foreground_color: Color::Black,
            background_color: Color::Cyan,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        });

        mode.insert("Insert".to_string(), ColorScheme {
            foreground_color: Color::Black,
            background_color: Color::Green,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        });

        mode.insert("Command".to_string(), ColorScheme {
            foreground_color: Color::Black,
            background_color: Color::Magenta,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        });
        
        Self {
            pane: ColorScheme::default(),
            ui: ColorScheme::default(),
            mode,
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

fn parse_custom_binding(value: &toml::Value) -> (Keys, Command) {
    let keys = parse_keys(&value["binding"]);

    let command = value["command"].as_str().unwrap().to_string();

    (keys[0].clone(), command)
}

fn parse_custom(value: &toml::Value) -> Vec<(Keys, Command)> {
    let mut custom = Vec::new();

    let array = value.as_array().expect("custom keybindings were not an array");

    for value in array {
        custom.push(parse_custom_binding(value));
    }

    custom
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
    if let Some(value) = table.get("custom") {
        let custom = parse_custom(value);

        for (keys, command) in custom {
            keybindings.insert(keys, command);
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

    let colors = match table.get("color") {
        None => EditorColors::default(),
        Some(value) => parse_editor_colors(value),
    };
    
    Settings {
        editor_settings,
        mode_keybindings,
        colors,
    }
}

fn parse_editor_colors(table: &toml::Value) -> EditorColors {
    let table = table.as_table().expect("editor colors were not a table");
    let mut editor_colors = EditorColors::default();

    match table.get("pane") {
        None => editor_colors.pane = ColorScheme::default(),
        Some(value) => editor_colors.pane = parse_color_scheme(value),
    }

    match table.get("ui") {
        None => editor_colors.ui = ColorScheme::default(),
        Some(value) => editor_colors.ui = parse_color_scheme(value),
    }

    editor_colors
}

fn parse_color_scheme(table: &toml::Value) -> ColorScheme {
    let table = table.as_table().expect("color scheme was not a table");
    let mut color_scheme = ColorScheme::default();
    
    match table.get("foreground_color") {
        None => color_scheme.foreground_color = Color::Reset,
        Some(value) => color_scheme.foreground_color = parse_color(value),
    }

    match table.get("background_color") {
        None => color_scheme.background_color = Color::Reset,
        Some(value) => color_scheme.background_color = parse_color(value),
    }

    match table.get("underline_color") {
        None => color_scheme.underline_color = Color::Reset,
        Some(value) => color_scheme.underline_color = parse_color(value),
    }

    match table.get("attributes") {
        None => color_scheme.attributes = Rc::new(Vec::new()),
        Some(value) => color_scheme.attributes = Rc::new(parse_attributes(value)),
    }

    color_scheme
}

fn parse_attributes(list: &toml::Value) -> Vec<Attribute> {
    let list = list.as_array().expect("attributes were not an array");

    let mut attributes = Vec::new();

    for attribute in list {
        let attribute = attribute.as_str().expect("attribute was not a string");

        match attribute {
            "reset" => attributes.push(Attribute::Reset),
            "bold" => attributes.push(Attribute::Bold),
            "dim" => attributes.push(Attribute::Dim),
            "italic" => attributes.push(Attribute::Italic),
            "underlined" => attributes.push(Attribute::Underlined),
            "double_underlined" => attributes.push(Attribute::DoubleUnderlined),
            "under_curled" => attributes.push(Attribute::Undercurled),
            "under_dotted" => attributes.push(Attribute::Underdotted),
            "under_dashed" => attributes.push(Attribute::Underdashed),
            "slow_blink" => attributes.push(Attribute::SlowBlink),
            "rapid_blink" => attributes.push(Attribute::RapidBlink),
            "reverse" => attributes.push(Attribute::Reverse),
            "hidden" => attributes.push(Attribute::Hidden),
            "crossed_out" => attributes.push(Attribute::CrossedOut),
            "fraktur" => attributes.push(Attribute::Fraktur),
            "no_bold" => attributes.push(Attribute::NoBold),
            "normal_intensity" => attributes.push(Attribute::NormalIntensity),
            "no_italic" => attributes.push(Attribute::NoItalic),
            "no_underline" => attributes.push(Attribute::NoUnderline),
            "no_blink" => attributes.push(Attribute::NoBlink),
            "no_reverse" => attributes.push(Attribute::NoReverse),
            "no_hidden" => attributes.push(Attribute::NoHidden),
            "not_crossed_out" => attributes.push(Attribute::NotCrossedOut),
            "framed" => attributes.push(Attribute::Framed),
            "encircled" => attributes.push(Attribute::Encircled),
            "overlined" => attributes.push(Attribute::OverLined),
            "not_framed_or_encircled" => attributes.push(Attribute::NotFramedOrEncircled),
            "not_overlined" => attributes.push(Attribute::NotOverLined),
            value => {
                panic!("unknown attribute: {}", value);
            }

        }

    }

    attributes
}

fn parse_color(value: &toml::Value) -> Color {

    if value.is_str() {
        let value = value.as_str().expect("color was not a string");

        match value {
            "black" => Color::Black,
            "red" => Color::Red,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "blue" => Color::Blue,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "white" => Color::White,
            "dark-grey" => Color::DarkGrey,
            "dark-red" => Color::DarkRed,
            "dark-green" => Color::DarkGreen,
            "dark-yellow" => Color::DarkYellow,
            "dark-blue" => Color::DarkBlue,
            "dark-magenta" => Color::DarkMagenta,
            "dark-cyan" => Color::DarkCyan,
            "grey" => Color::Grey,
            _ => unreachable!(),
        }
    }
    else if value.is_array() {
        let value = value.as_array().expect("color was not an array");

        if value.len() != 3 {
            panic!("color array was not of length 3");
        }

        Color::Rgb {
            r: value[0].as_integer().expect("color array value was not an integer") as u8,
            g: value[1].as_integer().expect("color array value was not an integer") as u8,
            b: value[2].as_integer().expect("color array value was not an integer") as u8,
        }
    }
    else {
        panic!("color was not a string or array");
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
