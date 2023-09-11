use std::{cell::RefCell, rc::Rc, collections::HashMap};

use crossterm::{style::Attribute, event::{KeyEvent, KeyModifiers, KeyCode}};

use crate::{settings::{Keys, Key}, pane::PaneContainer, window::StyledChar};

use super::{PromptType, Promptable, Mode};






pub struct DropDown {
    buttons: Rc<RefCell<PromptType>>,
    keybindings: Rc<RefCell<HashMap<Keys, String>>>,
}


impl Promptable for DropDown {
    fn draw_prompt(&mut self, row: usize, container: &PaneContainer, output: &mut Vec<Option<StyledChar>>) {
        let width = container.get_size().0;

        let buttons = self.buttons.borrow_mut();

        let color_settings = container.settings.borrow().colors.popup.clone();

        let button_str = buttons.draw_button(row).expect("Buttons were not buttons");

        match buttons {
            PromptType::Button(_, selected) => {
                
                let color_settings = if *selected == row {
                    color_settings.add_attribute(Attribute::Reverse)
                }
                else {
                    color_settings.clone()
                };

                for c in button_str.chars() {
                    output.push(Some(StyledChar::new(c, color_settings)));
                }
            },
            _ => panic!("Buttons were not buttons"),
        }
        
    }

    fn max_width(&self) -> usize {
        todo!()
    }

}


impl Mode for DropDown {
    fn get_name(&self) -> String {
        "Drop Down".to_string()
    }

    fn process_keypress(&mut self, key: crossterm::event::KeyEvent, pane: &mut dyn crate::pane::Pane, container: &mut PaneContainer) -> std::io::Result<bool> {

        match key {
            KeyEvent {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.execute_command("submit", pane, container);
                
                return Ok(true);
            },
            key_event => {
                let key = Key::from(key_event);
                let key = vec![key];

                if let Some(command) = self.keybindings.borrow().get(&key) {
                    self.execute_command(command, pane, container);
                }
                return Ok(true);
            }

        }
    }

    fn change_mode(&mut self, name: &str, pane: &mut dyn crate::pane::Pane, container: &mut PaneContainer) {
        todo!()
    }

    fn update_status(&mut self, pane: &dyn crate::pane::Pane, container: &PaneContainer) -> (String, String, String) {
        todo!()
    }

    fn add_keybindings(&mut self, bindings: HashMap<Keys, String>) {
        self.keybindings.borrow_mut().extend(bindings);
    }

    fn set_key_timeout(&mut self, timeout: u64) {
        todo!()
    }

    fn flush_key_buffer(&mut self) {
        todo!()
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

                match *buttons {
                    PromptType::Button(buttons, selected) => {
                        command.push_str(&format!("button {}", buttons[selected].1(self)));

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

        }
    }

    fn refresh(&mut self) {
        todo!()
    }

}
