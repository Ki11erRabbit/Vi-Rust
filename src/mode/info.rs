use std::{collections::HashMap, io};

use crossterm::event::KeyEvent;

use crate::{pane::{PaneContainer, Pane}, window::StyledChar, settings::Keys};

use super::{Promptable, Mode};




pub struct Info {
    body: Vec<Option<String>>,
}

impl Info {
    pub fn new(body: Vec<Option<String>>) -> Self {
        Self { body }
    }
}

impl Promptable for Info {

    fn draw_prompt(&mut self, row: usize, container: &PaneContainer) -> Vec<Option<StyledChar>> {
        let mut output = Vec::new();
    
        let width = container.get_size().0;


        let color_settings = container.get_settings().borrow().colors.popup.clone();

        match self.body.get(row) {
            Some(None) => {
                let gap = " ".repeat(width);
                for chr in gap.chars() {
                    output.push(Some(StyledChar::new(chr, color_settings.clone())));
                }
            },
            Some(Some(ref text)) => {
                for chr in text.chars() {
                    output.push(Some(StyledChar::new(chr, color_settings.clone())));
                }

                let gap = " ".repeat(width - text.len());

                for chr in gap.chars() {
                    output.push(Some(StyledChar::new(chr, color_settings.clone())));
                }
            },
            None => {},
        }

        output
    }

    fn max_width(&self) -> usize {
        let mut max = 0;
        for line in &self.body {
            match line {
                None => {},
                Some(ref text) => {
                    if text.len() > max {
                        max = text.len();
                    }
                }
            }
        }
        max
    }

    fn process_keypress(&mut self, key: crate::settings::Key, _pane: &mut dyn Pane, _container: &mut PaneContainer) {
    }
}

impl Mode for Info {
    fn get_name(&self) -> String {
        "Info".to_string()
    }


    fn add_keybindings(&mut self, _bindings: HashMap<Keys, String>) {
        // Do nothing
    }

    fn set_key_timeout(&mut self, _timeout: u64) {
        // Do nothing
    }

    fn flush_key_buffer(&mut self) {
        // Do nothing
    }

    fn refresh(&mut self) {
        // Do nothing
    }
}
