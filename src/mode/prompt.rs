use std::{io, collections::HashMap, cell::RefCell, rc::Rc};

use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};
use crossterm::style::Attribute;

use crate::pane::PaneContainer;
use crate::settings::{ColorScheme, Key};
use crate::{mode::Mode, pane::Pane, settings::Keys};
use crate::window::StyledChar;

use super::Promptable;




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

}



pub struct Prompt {
    /// The type of prompt to display on each line
    prompts: Rc<RefCell<Vec<PromptType>>>,
    keybindings: Rc<RefCell<HashMap<Keys, String>>>,
    current_prompt: usize,
}

impl Prompt {
    pub fn new(prompts: Vec<PromptType>) -> Self {
        Self {
            prompts: Rc::new(RefCell::new(prompts)),
            keybindings: Rc::new(RefCell::new(HashMap::new())),
            current_prompt: 0,
        }
    }
}


impl Promptable for Prompt {
    fn draw_prompt(&mut self, row: usize, container: &PaneContainer, output: &mut Vec<Option<StyledChar>>) {

        let width = container.get_size().0 - 2;// - 2 for the border
        
        let prompt = &self.prompts.borrow()[self.current_prompt];

        let color_settings = container.settings.borrow().colors.popup.clone();

        match prompt {
            PromptType::Text(_,size,_) => {
                let mut text = prompt.draw_text().unwrap();
                if text.chars().count() == 0 {
                    text = "_".repeat(width);
                }
                match size {
                    None => {
                        while text.chars().count() < width {
                            text.push('_');
                        }
                    },
                    Some(size) => {
                        while text.chars().count() < *size {
                            text.push('_');
                        }
                    },
                }

                let remaining = width.saturating_sub(text.chars().count());

                let mut side = remaining / 2;

                for _ in 0..side {
                    output.push(Some(StyledChar::new(' ', color_settings.clone())));
                }

                let text_color = if self.current_prompt == row {
                    ColorScheme::add_attribute(&color_settings.clone(), Attribute::Reverse)
                } else {
                    color_settings.clone()
                };
                
                for c in text.chars() {
                    output.push(Some(StyledChar::new(c, text_color.clone())));
                }

                if remaining % 2 != 0 {
                    side += 1;
                }

                for _ in 0..side {
                    output.push(Some(StyledChar::new(' ', color_settings.clone())));
                }
            },
            PromptType::Button(buttons, selected) => {
                let button_count = buttons.len();

                for i in 0..button_count {
                    let button = prompt.draw_button(i).unwrap();


                    for _ in 0..(width - button.chars().count() / button_count) {
                        output.push(Some(StyledChar::new(' ', color_settings.clone())));
                    }

                    let button_color = if i== *selected {
                        ColorScheme::add_attribute(&color_settings.clone(), Attribute::Reverse)
                    }
                    else {
                        color_settings.clone()
                    };

                    for c in button.chars() {
                        output.push(Some(StyledChar::new(c, button_color.clone())));
                    }

                    for _ in 0..(width - button.chars().count() / button_count) {
                        output.push(Some(StyledChar::new(' ', color_settings.clone())));
                    }

                }
            },
            PromptType::Checkbox(checkboxes, selected) => {
                let checkbox_count = checkboxes.len();

                for i in 0..checkbox_count {
                    let checkbox = prompt.draw_checkbox(i).unwrap();

                    
                    for _ in 0..(width - checkbox.chars().count() / checkbox_count) {
                        output.push(Some(StyledChar::new(' ', color_settings.clone())));
                    }

                    let checkbox_color = if i == *selected {
                        ColorScheme::add_attribute(&color_settings.clone(), Attribute::Reverse)
                    } else {
                        color_settings.clone()
                    };

                    for c in checkbox.chars() {
                        output.push(Some(StyledChar::new(c, checkbox_color.clone())));
                    }
                    
                    for _ in 0..(width - checkbox.chars().count() / checkbox_count) {
                        output.push(Some(StyledChar::new(' ', color_settings.clone())));
                    }

                }
            },
            PromptType::Radio(radios, selected, pos) => {
                let radio_count = radios.len();

                for i in 0..radio_count {
                    let radio = prompt.draw_radio(i).unwrap();

                    for _ in 0..(width - radio.chars().count() / radio_count) {
                        output.push(Some(StyledChar::new(' ', color_settings.clone())));
                    }

                    let radio_color = if i == selected.unwrap_or(*pos) {
                        ColorScheme::add_attribute(&color_settings.clone(), Attribute::Reverse)
                    } else {
                        color_settings.clone()
                    };

                    for c in radio.chars() {
                        output.push(Some(StyledChar::new(c, radio_color.clone())));
                    }

                   for _ in 0..(width - radio.chars().count() / radio_count) {
                        output.push(Some(StyledChar::new(' ', color_settings.clone())));
                   }

                }
            },
            

        }
    }

    fn max_width(&self) -> usize {
        let mut max = 0;
        let prompts = self.prompts.clone();
        for prompt in prompts.borrow().iter() {
            match prompt {
                PromptType::Text(text, len, _) => {
                    if let Some(len) = len {
                        max = max.max(*len + 2);
                    }
                    max = max.max(text.chars().count() + 2);
                },
                PromptType::Button(buttons, _) => {
                    let mut total = 0;
                    for button in buttons {
                        total += button.0.chars().count() + 2;
                    }

                    max = max.max(total);
                },
                PromptType::Checkbox(checkboxes, _) => {
                    let mut total = 0;
                    for i in 0..checkboxes.len() {
                        total += prompt.draw_checkbox(i).unwrap().chars().count() + 2;
                    }

                    max = max.max(total);
                },
                PromptType::Radio(radios, _, _) => {
                    let mut total = 0;
                    for i in 0..radios.len() {
                        total += prompt.draw_radio(i).unwrap().chars().count() + 2;
                    }

                    max = max.max(total);
                },
            }
        }

        max
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
        //self.timeout = timeout;
    }

    fn flush_key_buffer(&mut self) {
        //self.key_buffer.clear();
    }

    fn refresh(&mut self) {
    }

    fn change_mode(&mut self, mode: &str, pane: &mut dyn Pane, container: &mut PaneContainer) {
    }

    fn update_status(&mut self, pane: &dyn Pane, container: &PaneContainer) -> (String, String, String) {
        ("".to_string(), "".to_string(), "".to_string())
    }

    fn execute_command(&mut self, command: &str, pane: &mut dyn Pane, container: &mut PaneContainer) {
        match command {
            "cancel" => {
                pane.run_command("cancel", container);
            },
            "submit" => {
                let mut command = String::from("submit ");

                let prompts = self.prompts.clone();
                let prompts = prompts.borrow();
                match &prompts[self.current_prompt] {
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
                        
                        command.push_str(&radios[selected.unwrap()]);
                    },
                }

                eprintln!("{}", command);

                pane.run_command(&command, container);
            },
            "toggle" => {
                let prompts = self.prompts.clone();
                let mut prompts = prompts.borrow_mut();
                match &mut prompts[self.current_prompt] {
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

                        pane.run_command(&command, container);
                    },
                    _ => {},
                }
            },
            "left" => {
                let prompts = self.prompts.clone();
                let mut prompts = prompts.borrow_mut();
                match &mut prompts[self.current_prompt] {
                    PromptType::Button(buttons, selected) => {
                        *selected = selected.saturating_sub(1);
                    },
                    PromptType::Checkbox(checkboxes, selected) => {
                        *selected = selected.saturating_sub(1);
                    },
                    PromptType::Radio(radios, _, selected) => {
                        *selected = selected.saturating_sub(1);
                    },
                    _ => {},
                }
            },
            "right" => {
                let prompts = self.prompts.clone();
                let mut prompts = prompts.borrow_mut();
                match &mut prompts[self.current_prompt] {
                    PromptType::Button(buttons, selected) => {
                        *selected = (*selected + 1).min(buttons.len() - 1);
                    },
                    PromptType::Checkbox(checkboxes, selected) => {
                        *selected = (*selected + 1).min(checkboxes.len() - 1);
                    },
                    PromptType::Radio(radios, _, selected) => {
                        *selected = (*selected + 1).min(radios.len() - 1);
                    },
                    _ => {},
                }
            },
            "up" => {
                self.current_prompt = self.current_prompt.saturating_sub(1);
            },
            "down" => {
                self.current_prompt = (self.current_prompt + 1).min(self.prompts.borrow().len() - 1);
            },
            _ => {},
        }

    }


    fn process_keypress(&mut self, key: KeyEvent, pane: &mut dyn Pane, container: &mut PaneContainer) -> io::Result<bool> {

        match key {
            KeyEvent {
                code: code @ KeyCode::Char(..),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                ..
            } => {
                if code == KeyCode::Char(' ') {
                    self.execute_command("toggle", pane, container);
                }
                else {
                    let prompts = self.prompts.clone();
                    let mut prompts = prompts.borrow_mut();
                    match &mut prompts[self.current_prompt] {
                        PromptType::Text(text, limit, _) => {

                            let chr = match code {
                                KeyCode::Char(chr) => chr,
                                _ => return Ok(true),
                            };
                            match limit {
                                Some(limit) => {
                                    
                                    if text.chars().count() < *limit {
                                        text.push(chr);
                                    }
                                },
                                None => {
                                    text.push(chr);
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
                let prompts = self.prompts.clone();
                match &mut prompts.borrow_mut()[self.current_prompt] {
                    PromptType::Text(text, _, _) => {
                        text.pop();
                    },
                    _ => {},
                }
                return Ok(true);
            },
            key_event => {
                let key = Key::from(key_event);
                let key = vec![key];

                if let Some(command) = self.keybindings.clone().borrow().get(&key) {
                    self.execute_command(command,pane, container);
                }

                return Ok(true);
            }
        }
    }

}
