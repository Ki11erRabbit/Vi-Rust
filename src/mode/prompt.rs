use std::{io, collections::HashMap, cell::RefCell, rc::Rc};

use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};

use crate::pane::PaneContainer;
use crate::{mode::Mode, pane::Pane, settings::Keys};
use crate::window::OutputSegment;


pub trait Promptable {
    fn draw_prompt(&mut self, row: usize, container: &PaneContainer) -> Vec<OutputSegment>;

    fn max_width(&self) -> usize;
}


pub enum PromptType {
    /// A prompt that takes a single line of text
    /// The optional usize is the maximum length of the input, none means no limit
    /// The bool is whether or not to hide the input
    Text(String, Option<usize>,bool),
    /// A button that has a label and a function to call when pressed
    /// The usize is the index of the currently selected button
    Button(Vec<(String, Box<dyn Fn(&Prompt) -> String>)>, usize),
    /// A checkbox that has a label and a bool indicating whether or not it is checked
    /// The usize is the index of the currently selected checkbox
    Checkbox(Vec<(String, bool)>, usize),
    /// A radio button has a label and the optional usize indicating which option is currently active
    /// The usize is the index of the currently selected radio button
    Radio(Vec<String>, Option<usize>, usize),
}


pub struct Prompt {
    /// The type of prompt to display on each line
    prompts: Vec<PromptType>,
    keybindings: Rc<RefCell<HashMap<Keys, String>>>,
    current_prompt: usize,
}

impl Prompt {
    pub fn new(prompts: Vec<PromptType>) -> Self {
        Self {
            prompts,
            keybindings: Rc::new(RefCell::new(HashMap::new())),
            current_prompt: 0,
        }
    }
}

impl Mode for Prompt {
    fn get_name(&self) -> String {
        "prompt".to_string()
    }

    fn add_keybindings(&mut self, bindings: HashMap<Keys, String>) {
        self.keybindings.borrow_mut().extend(bindings);
    }

    fn set_key_timeout(&mut self, timeout: u64) {
        self.timeout = timeout;
    }

    fn flush_key_buffer(&mut self) {
        self.key_buffer.clear();
    }

    fn refresh(&mut self) {
    }

    fn change_mode(&mut self, mode: &str, pane: &mut dyn Pane) {
    }

    fn update_status(&mut self, pane: &PaneContainer) -> (String, String, String) {
        ("".to_string(), "".to_string(), "".to_string())
    }

    fn execute_command(&mut self, command: &str, pane: &mut dyn Pane) {
        match command {
            "cancel" => {
                pane.run_command("cancel");
            },
            "submit" => {
                let mut command = String::from("submit ");

                match &self.prompts[self.current_prompt] {
                    PromptType::Text(text, _, _) => {
                        command.push_str(&format!("text {}", text));
                    },
                    PromptType::Button(buttons, selected) => {
                        command.push_str(&format!("button {}", buttons[*selected].1(self)));
                    },
                    PromptType::Checkbox(checkboxes, selected) => {
                        command.push_str("checkbox ");
                        for (label, checked) in checkboxes {
                            command.push_str(&format!("{}={},", label, checked));
                        }
                    },
                    PromptType::Radio(radios, selected, _) => {
                        command.push_str("radio ");

                        match selected {
                            None => {
                                command.push_str("");
                            },
                            Some(selected) => {
                                command.push_str(&radios[*selected]);
                            },
                        }
                        
                        command.push_str(&radios[*selected.unwrap()]);
                    },
                }

                pane.run_command(&command);
            },
            "toggle" => {
                match &mut self.prompts[self.current_prompt] {
                    PromptType::Checkbox(checkboxes, selected) => {
                        checkboxes[*selected].1 = !checkboxes[*selected].1;
                    },
                    PromptType::Radio(radios, selected, pos) => {
                        *selected = Some(*pos);
                    },
                    PromptType::Text(text, _, _) => {
                        text.push_str(" ");
                    },
                    PromptType::Button(buttons, selected) => {
                        let mut command = String::from("submit ");
                        command.push_str(&format!("button {}", buttons[*selected].1(self)));

                        pane.run_command(&command);
                    },
                    _ => {},
                }
            },
            "left" => {
                match &mut self.prompts[self.current_prompt] {
                    PromptType::Button(buttons, selected) => {
                        *selected = selected.saturating_sub(1);
                    },
                    PromptType::Checkbox(checkboxes, selected) => {
                        *selected = selected.saturating_sub(1);
                    },
                    PromptType::Radio(radios, selected, _) => {
                        *selected = selected.saturating_sub(1);
                    },
                    _ => {},
                }
            },
            "right" => {
                match &mut self.prompts[self.current_prompt] {
                    PromptType::Button(buttons, selected) => {
                        *selected = (*selected + 1).min(buttons.len() - 1);
                    },
                    PromptType::Checkbox(checkboxes, selected) => {
                        *selected = (*selected + 1).min(checkboxes.len() - 1);
                    },
                    PromptType::Radio(radios, selected, _) => {
                        *selected = (*selected + 1).min(radios.len() - 1);
                    },
                    _ => {},
                }
            },
            "up" => {
                self.current_prompt = self.current_prompt.saturating_sub(1);
            },
            "down" => {
                self.current_prompt = (self.current_prompt + 1).min(self.prompts.len() - 1);
            },
            _ => {},
        }

    }


    fn process_keypress(&mut self, key: KeyEvent, pane: &mut dyn Pane) -> io::Result<bool> {

        match key {
            KeyEvent {
                code: code @ KeyCode::Char(..),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                ..
            } => {
                if code == KeyCode::Char(' ') {
                    self.execute_command("toggle", pane);
                }
                else {
                    match &mut self.prompts[self.current_prompt] {
                        PromptType::Text(text, limit, _) => {
                            match limit {
                                Some(limit) => {
                                    if text.chars().count() < *limit {
                                        text.push(code.into());
                                    }
                                },
                                None => {
                                    text.push(code.into());
                                },
                            }
                        },
                        _ => {},
                    }
                }
                return Ok(true);
            },
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                match &mut self.prompts[self.current_prompt] {
                    PromptType::Text(text, _, _) => {
                        text.pop();
                    },
                    _ => {},
                }
                return Ok(true);
            },
            key_event => {
                let key = Keys::from(key_event);

                if let Some(command) = self.keybindings.borrow().get(&key) {
                    self.execute_command(command, pane);
                }

                return Ok(true);
            }
        }
    }

}
