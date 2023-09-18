
pub mod base;
pub mod prompt;
pub mod info;
pub mod drop_down;


use std::{io, collections::HashMap};

use crossterm::event::KeyEvent;

use crate::{pane::{Pane, PaneContainer}, settings::Keys, window::StyledChar};





pub trait Mode {

    fn get_name(&self) -> String;


    fn change_mode(&mut self, name: &str, pane: &mut dyn Pane, container: &mut PaneContainer);


    fn add_keybindings(&mut self, bindings: HashMap<Keys, String>);

    fn set_key_timeout(&mut self, timeout: u64);

    fn flush_key_buffer(&mut self);


    fn refresh(&mut self);
}


pub trait TextMode {

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut dyn Pane, container: &mut PaneContainer) -> io::Result<()>;
    fn update_status(&mut self, pane: &dyn Pane, container: &PaneContainer) -> (String, String, String);

    fn execute_command(&mut self, command: &str, pane: &mut dyn Pane, pane: &mut PaneContainer);
}

pub trait Promptable: Mode {
    fn draw_prompt(&mut self, row: usize, container: &PaneContainer) -> Vec<Option<StyledChar>>;

    fn max_width(&self) -> usize;
}




pub enum PromptType {
    /// A prompt that takes a single line of text
    /// The optional usize is the maximum length of the input, none means no limit
    /// The bool is whether or not to hide the input
    Text(String, Option<usize>,bool),
    /// A button that has a label and a function to call when pressed
    /// The usize is the index of the currently selected button
    Button(Vec<(String, Box<dyn Fn(&dyn Promptable) -> String>)>, usize),
    /// A checkbox that has a label and a bool indicating whether or not it is checked
    /// The usize is the index of the currently selected checkbox
    Checkbox(Vec<(String, bool)>, usize),
    /// A radio button has a label and the optional usize indicating which option is currently active
    /// The usize is the index of the currently selected radio button
    Radio(Vec<String>, Option<usize>, usize),
}

impl PromptType {
    pub fn is_text(&self) -> bool {
        match self {
            PromptType::Text(_, _, _) => true,
            _ => false
        }
    }
    pub fn is_button(&self) -> bool {
        match self {
            PromptType::Button(_, _) => true,
            _ => false
        }
    }
    pub fn is_checkbox(&self) -> bool {
        match self {
            PromptType::Checkbox(_, _) => true,
            _ => false
        }
    }
    pub fn is_radio(&self) -> bool {
        match self {
            PromptType::Radio(_, _, _) => true,
            _ => false
        }
    }
    
    pub fn draw_text(&self) -> Option<String> {
        match self {
            PromptType::Text(text, len, hide) => {
                let count = text.chars().count();
                let mut output = if *hide {
                    format!("{}", "*".repeat(count))
                } else {
                    format!("{}", text)
                };
                if let Some(len) = len {
                    output.push_str("_".repeat(len - count).as_str());
                }

                Some(output)
                
            },
            _ => None
            
        }
    }

    pub fn draw_button(&self, index: usize) -> Option<String> {
        match self {
            PromptType::Button(buttons, _) => {
                if index >= buttons.len() {
                    return None;
                }

                let output = buttons[index].0.clone();

                Some(output)
            },
            _ => None
        }
    }

    pub fn draw_checkbox(&self, index: usize) -> Option<String> {
        match self {
            PromptType::Checkbox(checkboxes, selected) => {
                if index >= checkboxes.len() {
                    return None;
                }

                let output = if index == *selected {
                    format!("[x] {}", checkboxes[index].0)
                } else {
                    format!("[ ] {}", checkboxes[index].0)
                };

                Some(output)
            },
            _ => None
        }
    }

    pub fn draw_radio(&self, index: usize) -> Option<String> {
        match self {
            PromptType::Radio(radios, selected, pos) => {
                if index >= radios.len() {
                    return None;
                }

                let output = if index == selected.unwrap_or(*pos) {
                    format!("(*) {}", radios[index])
                } else {
                    format!("( ) {}", radios[index])
                };

                Some(output)
            },
            _ => None
        }
    }

    pub fn button_len(&self) -> Option<usize> {
        match self {
            PromptType::Button(buttons, _) => {
                let mut max = 0;
                for button in buttons {
                    if button.0.chars().count() > max {
                        max = button.0.chars().count();
                    }
                }

                Some(max)
            },
            _ => None
        }
    }

}
