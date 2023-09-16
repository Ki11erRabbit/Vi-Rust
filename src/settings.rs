use core::fmt;
use std::{collections::{HashMap, HashSet}, rc::Rc};

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

impl Settings {
    fn generate_normal_keybindings(normal_keybindings: &mut HashMap<Keys, Command>) {
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

        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('\\'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('j'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('t'),
            modifier: KeyModifiers::NONE,
        }], "prompt_jump".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('\\'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('j'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('s'),
            modifier: KeyModifiers::NONE,
        }], "prompt_set_jump".to_string());

        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('u'),
            modifier: KeyModifiers::NONE,
        }], "undo".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('r'),
            modifier: KeyModifiers::CONTROL,
        }], "redo".to_string());

        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::ALT,
        }, Key {
            key: KeyCode::Char('h'),
            modifier: KeyModifiers::NONE,
        }], "change_tab prev".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::ALT,
        }, Key {
            key: KeyCode::Char('l'),
            modifier: KeyModifiers::NONE,
        }], "change_tab next".to_string());

        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::ALT,
        }, Key {
            key: KeyCode::Char('t'),
            modifier: KeyModifiers::NONE,
        }], "open_tab".to_string());

        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::ALT,
        }, Key {
            key: KeyCode::Char('T'),
            modifier: KeyModifiers::NONE,
        }], "open_tab_with_pane".to_string());

        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('\\'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('i'),
            modifier: KeyModifiers::NONE,
        }], "info".to_string());

        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('\\'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('g'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('t'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('d'),
            modifier: KeyModifiers::NONE,
        }], "goto_definition".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('\\'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('g'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('t'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('D'),
            modifier: KeyModifiers::NONE,
        }], "goto_declaration".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('\\'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('g'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('t'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('t'),
            modifier: KeyModifiers::NONE,
        }], "goto_type_definition".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('\\'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('g'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('t'),
            modifier: KeyModifiers::NONE,
        }, Key {
            key: KeyCode::Char('i'),
            modifier: KeyModifiers::NONE,
        }], "goto_implementation".to_string());

        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('p'),
            modifier: KeyModifiers::NONE,
        }], "paste_after".to_string());
        normal_keybindings.insert(vec![Key {
            key: KeyCode::Char('P'),
            modifier: KeyModifiers::NONE,
        }], "paste_before".to_string());
                                       

    }

    fn generate_insert_keybindings(insert_keybindings: &mut HashMap<Keys, Command>) {
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


        insert_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::ALT,
        }, Key {
            key: KeyCode::Char('h'),
            modifier: KeyModifiers::NONE,
        }], "change_tab prev".to_string());
        insert_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::ALT,
        }, Key {
            key: KeyCode::Char('l'),
            modifier: KeyModifiers::NONE,
        }], "change_tab next".to_string());

        insert_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::ALT,
        }, Key {
            key: KeyCode::Char('t'),
            modifier: KeyModifiers::NONE,
        }], "open_tab".to_string());

        insert_keybindings.insert(vec![Key {
            key: KeyCode::Char('w'),
            modifier: KeyModifiers::ALT,
        }, Key {
            key: KeyCode::Char('T'),
            modifier: KeyModifiers::NONE,
        }], "open_tab_with_pane".to_string());

        insert_keybindings.insert(vec![Key {
            key: KeyCode::Char('n'),
            modifier: KeyModifiers::CONTROL,
        }], "completion".to_string());


    }

    fn generate_command_keybindings(command_keybindings: &mut HashMap<Keys, Command>) {
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

    }

    fn generate_prompt_keybindings(prompt_keybindings: &mut HashMap<Keys, Command>) {
        prompt_keybindings.insert(vec![Key {
            key: KeyCode::Esc,
            modifier: KeyModifiers::NONE,
        }], "cancel".to_string());
        prompt_keybindings.insert(vec![Key {
            key: KeyCode::Enter,
            modifier: KeyModifiers::NONE,
        }], "submit".to_string());
        prompt_keybindings.insert(vec![Key {
            key: KeyCode::Char(' '),
            modifier: KeyModifiers::NONE,
        }], "toggle".to_string());
        prompt_keybindings.insert(vec![Key {
            key: KeyCode::Left,
            modifier: KeyModifiers::NONE,
        }], "left".to_string());
        prompt_keybindings.insert(vec![Key {
            key: KeyCode::Right,
            modifier: KeyModifiers::NONE,
        }], "right".to_string());

    }

    fn generate_drop_down_keybindings(drop_down_keybindings: &mut HashMap<Keys, Command>) {
        drop_down_keybindings.insert(vec![Key {
            key: KeyCode::Esc,
            modifier: KeyModifiers::NONE,
        }], "cancel".to_string());
        drop_down_keybindings.insert(vec![Key {
            key: KeyCode::Enter,
            modifier: KeyModifiers::NONE,
        }], "submit".to_string());
        drop_down_keybindings.insert(vec![Key {
            key: KeyCode::Char(' '),
            modifier: KeyModifiers::NONE,
        }], "submit".to_string());
        drop_down_keybindings.insert(vec![Key {
            key: KeyCode::Up,
            modifier: KeyModifiers::NONE,
        }], "up".to_string());
        drop_down_keybindings.insert(vec![Key {
            key: KeyCode::Down,
            modifier: KeyModifiers::NONE,
        }], "down".to_string());
    }
}

impl Default for Settings {
    fn default() -> Self {
        let editor_settings = EditorSettings::default();
        let mut mode_keybindings = HashMap::new();

        let mut normal_keybindings = HashMap::new();

        Self::generate_normal_keybindings(&mut normal_keybindings);

        
        let mut insert_keybindings = HashMap::new();

        Self::generate_insert_keybindings(&mut insert_keybindings);


        let mut command_keybindings = HashMap::new();

        Self::generate_command_keybindings(&mut command_keybindings);


        mode_keybindings.insert("Normal".to_string(), normal_keybindings);
        mode_keybindings.insert("Insert".to_string(), insert_keybindings);
        mode_keybindings.insert("Command".to_string(), command_keybindings);

        let mut prompt_keybindings = HashMap::new();

        Self::generate_prompt_keybindings(&mut prompt_keybindings);

        mode_keybindings.insert("Prompt".to_string(), prompt_keybindings);

        let mut drop_down_keybindings = HashMap::new();

        Self::generate_drop_down_keybindings(&mut drop_down_keybindings);

        mode_keybindings.insert("Drop Down".to_string(), drop_down_keybindings);

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
    pub rainbow_delimiters: bool,
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
            rainbow_delimiters: true,
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
        let mut attributes = (*self.attributes).clone();
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
    /// The color scheme for all panes.
    pub pane: ColorScheme,
    /// The color scheme for the ui.
    /// This includes line numbers and the status bar.
    pub ui: ColorScheme,
    /// The color scheme for the status bar.
    pub bar: ColorScheme,
    /// The color scheme for popup panes.
    pub popup: ColorScheme,
    /// The color scheme for the Currently selected mode.
    pub mode: HashMap<String, ColorScheme>,
    /// The color scheme for treesitter nodes.
    pub treesitter: Rc<HashMap<String,HashMap<String, SyntaxHighlight>>>,
    pub rainbow_delimiters: Vec<ColorScheme>,
}


impl Default for EditorColors {
    fn default() -> Self {
        let mut mode = HashMap::new();

        mode.insert("Normal".to_string(), ColorScheme {
            foreground_color: Color::Black,
            background_color: Color::DarkCyan,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        });

        mode.insert("Insert".to_string(), ColorScheme {
            foreground_color: Color::Black,
            background_color: Color::DarkGreen,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        });

        mode.insert("Command".to_string(), ColorScheme {
            foreground_color: Color::Black,
            background_color: Color::DarkMagenta,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        });

        let mut treesitter = HashMap::new();

        Self::generate_scheme_colors(&mut treesitter);
        Self::generate_rust_colors(&mut treesitter);
        Self::generate_c_colors(&mut treesitter);

        


        let treesitter = Rc::new(treesitter);


        let mut rainbow_delimiters = Vec::new();

        rainbow_delimiters.push(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        });
        rainbow_delimiters.push(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        });
        rainbow_delimiters.push(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        });
        rainbow_delimiters.push(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        });
        rainbow_delimiters.push(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        });
        rainbow_delimiters.push(ColorScheme {
            foreground_color: Color::DarkRed,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        });

        let ui = ColorScheme {
            foreground_color: Color::DarkGrey,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        };
        
        Self {
            pane: ColorScheme::default(),
            ui,
            bar: ColorScheme::default(),
            popup: ColorScheme {
                foreground_color: Color::White,
                background_color: Color::DarkGrey,
                underline_color: Color::Reset,
                attributes: Rc::new(Vec::new()),
            },
            mode,
            treesitter,
            rainbow_delimiters,
        }
    }
}

impl EditorColors {
    fn generate_rust_colors(treesitter: &mut HashMap<String, HashMap<String, SyntaxHighlight>>) {

        let mut rust = HashMap::new();
        
        rust.insert("array_expression".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("async_block".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("await_expression".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("block".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("char_literal".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("crate".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("escape_sequence".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("line_comment".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGrey,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("block_comment".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGrey,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("metavariable".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("mutable_specifier".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("primitive_type".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("raw_string_literal".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("string_literal".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("self".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("super".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("type_identifier".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("as".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("async".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("await".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("break".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("continue".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("const".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("default".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("dyn".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("else".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("extern".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("false".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("true".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("fn".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("for".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("ident".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("if".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("impl".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("in".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("let".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("item".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Reset,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("lifetime".to_string(),SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkRed,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("loop".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("macro_rules!".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        rust.insert("match".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("move".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        rust.insert("pub".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("return".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("struct".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("enum".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("trait".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("type".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("union".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("unsafe".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        rust.insert("use".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("where".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        rust.insert("vis".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("while".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        rust.insert("mod".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));

        let mut function_item = HashMap::new();

        function_item.insert("name".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));

        rust.insert("function_item".to_string(), SyntaxHighlight::Parent(
            function_item,
        ));

        let mut macro_invocation = HashMap::new();

        macro_invocation.insert("macro".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));

        rust.insert("macro_invocation".to_string(), SyntaxHighlight::Parent(
            macro_invocation
        ));

        let mut scoped_identifier = HashMap::new();

        scoped_identifier.insert("path".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));

        let mut scoped_identifier2 = HashMap::new();

        scoped_identifier2.insert("call_expression".to_string(), SyntaxHighlight::Parent(
            scoped_identifier
        ));

        let mut scoped_identifier = HashMap::new();
        scoped_identifier.insert("path".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Magenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));

        scoped_identifier2.insert("use_declaration".to_string(), SyntaxHighlight::Parent(
            scoped_identifier
        ));
        
        rust.insert("scoped_identifier".to_string(), SyntaxHighlight::GrandParent(
            scoped_identifier2
        ));

        let mut tuple_struct_pattern = HashMap::new();

        tuple_struct_pattern.insert("type".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));

        rust.insert("tuple_struct_pattern".to_string(), SyntaxHighlight::Parent(
            tuple_struct_pattern
        ));
        
        
        rust.insert("macro_invocation".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
            }));

        rust.insert("try_expression".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
            }));

        /*rust.insert("scoped_identifier".to_string(), SyntaxHighlight {
            color: SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
    })});*/

        let mut field_declaration = HashMap::new();

        field_declaration.insert("name".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Magenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));

        rust.insert("field_declaration".to_string(), SyntaxHighlight::Parent(
            field_declaration,
        ));

        let mut exclude = HashSet::new();
        exclude.insert(',');
        
        rust.insert("use_list".to_string(), SyntaxHighlight::ChildExclude(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        },exclude));
                                                  

        let mut let_declaration = HashMap::new();

        let_declaration.insert("pattern".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Magenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));

        rust.insert("let_declaration".to_string(), SyntaxHighlight::Parent(
            let_declaration,
        ));

        rust.insert("ERROR".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkRed,
            background_color: Color::Reset,
            underline_color: Color::Red,
            attributes: Rc::new(vec![Attribute::Undercurled]),
        }));

        treesitter.insert("rust".to_string(), rust);
        
    }

    fn generate_c_colors(treesitter: &mut HashMap<String, HashMap<String, SyntaxHighlight>>) {

        let mut c = HashMap::new();

        c.insert("#define".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("#include".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("#elif".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("#else".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("#endif".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("#if".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("#ifdef".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("#ifndef".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("#elifdef".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("#elifndef".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("#pragma".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("_Alignof".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("_Atomic".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("_Generic".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("_Noreturn".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("_alignof".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__alignof__".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__asm__".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__attribute__".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__based".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__cdecl".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__clrcall".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__declspec".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__fastcall".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__forceinline".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__extension__".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("__inline".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__inline__".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__restrict__".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("__stdcall".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__thiscall".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__thread".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("__unaligned".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("__vectorcall".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("_alignof".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("_unaligned".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("defined".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));
        c.insert("preproc_arg".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("preproc_directive".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(vec![Attribute::Bold]),
        }));



        c.insert("alignof".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("asm".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("offsetof".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("inline".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("auto".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("break".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("continue".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("case".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("default".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("const".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("constexpr".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("do".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("else".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("enum".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("extern".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("for".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("goto".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("if".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("long".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("noreturn".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("primitive_type".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("register".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("restrict".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("return".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("short".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("signed".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("sizeof".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("static".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("struct".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("switch".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("thread_local".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("typedef".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkCyan,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("union".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("unsigned".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("volatile".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("while".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        
        

        
        c.insert("statement_identifier".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkRed,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("system_lib_string".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        
        
        
        c.insert("char_literal".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("string_literal".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("string_content".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("character".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("comment".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGrey,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("escape_sequence".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGreen,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("false".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("true".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("nullptr".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkBlue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("number_literal".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Reset,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));

        c.insert("ms_restrict_modifier".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("ms_signed_ptr_modifier".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));
        c.insert("ms_unsigned_ptr_modifier".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));

        let mut field_declaration = HashMap::new();

        field_declaration.insert("declarator".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Magenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));

        c.insert("field_declaration".to_string(), SyntaxHighlight::Parent(
            field_declaration
        ));

        let mut function_declarator = HashMap::new();

        function_declarator.insert("declarator".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkMagenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));

        c.insert("function_declarator".to_string(), SyntaxHighlight::Parent(
            function_declarator
        ));


        let mut struct_specifier = HashMap::new();

        struct_specifier.insert("name".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkYellow,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));

        c.insert("struct_specifier".to_string(), SyntaxHighlight::Parent(
            struct_specifier
        ));
         

        c.insert("ERROR".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkRed,
            background_color: Color::Reset,
            underline_color: Color::Red,
            attributes: Rc::new(vec![Attribute::Undercurled]),
        }));

        //field_identifier
        //identifer
        //type_identifier



        
        treesitter.insert("c".to_string(), c);
    }
    
    fn generate_scheme_colors(treesitter: &mut HashMap<String, HashMap<String, SyntaxHighlight>>) {

        let mut scheme = HashMap::new();
        
        scheme.insert("comment".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGrey,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
            }));
        scheme.insert("block_comment".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGrey,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
            }));
        scheme.insert("directive".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::DarkGrey,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
            }));
        scheme.insert("symbol".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Reset,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
            }));
        scheme.insert("keyword".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Blue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
            }));
        scheme.insert("list".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Blue,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
            }));
        scheme.insert("boolean".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Magenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
            }));
        scheme.insert("character".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Magenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
            }));
        scheme.insert("string".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Green,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
            }));
        scheme.insert("escape_sequence".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Green,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
            }));
        
        scheme.insert("number".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Magenta,
            background_color: Color::Reset,
            underline_color: Color::Reset,
            attributes: Rc::new(Vec::new()),
        }));

        scheme.insert("ERROR".to_string(), SyntaxHighlight::Child(ColorScheme {
            foreground_color: Color::Red,
            background_color: Color::Reset,
            underline_color: Color::Red,
            attributes: Rc::new(vec![Attribute::Undercurled]),
        }));


        treesitter.insert("scheme".to_string(), scheme);
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxHighlight {
    /// A single color scheme
    Child(ColorScheme),
    /// A single color scheme with a set of characters to exclude
    ChildExclude(ColorScheme,HashSet<char>),
    /// A color scheme to be applied on a child via the parent's type
    Parent(HashMap<String, SyntaxHighlight>),
    /// A color scheme to be applied on a child via the parent's type with a set of characters to exclude
    ParentExclude(HashMap<String, SyntaxHighlight>,HashSet<char>),
    /// A Syntax Highlight to be applied on a node based on the first matching ancestor
    GrandParent(HashMap<String, SyntaxHighlight>),
        
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
