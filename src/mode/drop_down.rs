use std::{cell::RefCell, rc::Rc, collections::HashMap};

use crossterm::{style::Attribute, event::{KeyEvent, KeyModifiers, KeyCode}};

use crate::{settings::{Keys, Key}, pane::PaneContainer, window::StyledChar};

use super::{PromptType, Promptable, Mode};






pub struct DropDown {
    buttons: Rc<RefCell<PromptType>>,
    keybindings: Rc<RefCell<HashMap<Keys, String>>>,
}

impl DropDown {
    pub fn new(buttons: PromptType) -> Self {
        Self {
            buttons: Rc::new(RefCell::new(buttons)),
            keybindings: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    fn execute_command(&mut self, command: &str, pane: &mut dyn crate::pane::Pane, container: &mut PaneContainer) {
        match command {
            "cancel" => {
                pane.run_command("cancel", container);
            },
            "submit" => {
                let mut command = String::from("submit ");

                let buttons = self.buttons.clone();
                let buttons = buttons.borrow();

                match &*buttons {
                    PromptType::Button(buttons, selected) => {
                        command.push_str(&format!("button {}", buttons[*selected].1(self)));

                        pane.run_command(&command, container);
                    }
                    _ => panic!("Buttons were not buttons"),
                }
            },
            "up" => {
                let mut buttons = self.buttons.borrow_mut();

                match &mut *buttons {
                    PromptType::Button(_,ref mut selected) => {
                        if *selected > 0 {
                            *selected -= 1;
                        }
                    }
                    _ => panic!("Buttons were not buttons"),
                }
            },
            "down" => {
                let mut buttons = self.buttons.borrow_mut();

                match &mut *buttons {
                    PromptType::Button(buttons, ref mut selected) => {
                        if *selected < buttons.len() - 1 {
                            *selected += 1;
                        }
                    }
                    _ => panic!("Buttons were not buttons"),
                }
            },
            _ => {}

        }
    }
    
}


impl Promptable for DropDown {
    fn draw_prompt(&mut self, row: usize, container: &PaneContainer) -> Vec<Option<StyledChar>> {
        let mut output = Vec::new();
        let width = container.get_size().0;

        let mut buttons = self.buttons.borrow_mut();

        let color_settings = container.get_settings().borrow().colors.popup.clone();

        let button_str = match buttons.draw_button(row) {
            Some(s) => s,
            None =>  {
                " ".repeat(width)
                .chars()
                .for_each(|c|
                          output.push(Some(StyledChar::new(c, color_settings.clone()))));
                return output;
            },
        };

        match &mut*buttons {
            PromptType::Button(_, selected) => {
                
                let color_settings = if *selected == row {
                    color_settings.add_attribute(Attribute::Reverse)
                }
                else {
                    color_settings.clone()
                };

                for c in button_str.chars() {
                    output.push(Some(StyledChar::new(c, color_settings.clone())));
                }
            },
            _ => panic!("Buttons were not buttons"),
        }

        output
    }

    fn max_width(&self) -> usize {
        todo!()
    }

    fn process_keypress(&mut self, key: Key, pane: &mut dyn crate::pane::Pane, container: &mut PaneContainer) {
        match key {
            Key {
                key: KeyCode::Char(' '),
                modifier: KeyModifiers::NONE,
                ..
            } => {
                self.execute_command("submit", pane, container);
                
            },
            key_event => {
                let key = Key::from(key_event);
                let key = vec![key];

                let keybindings = self.keybindings.clone();
                let keybindings = keybindings.borrow();

                if let Some(command) = keybindings.get(&key) {
                    self.execute_command(command, pane, container);
                }
            }

        }
    }

}


impl Mode for DropDown {
    fn get_name(&self) -> String {
        "Drop Down".to_string()
    }


    fn add_keybindings(&mut self, bindings: HashMap<Keys, String>) {
        self.keybindings.borrow_mut().extend(bindings);
    }

    fn set_key_timeout(&mut self, _timeout: u64) {
    }

    fn flush_key_buffer(&mut self) {
    }


    fn refresh(&mut self) {
    }

}
