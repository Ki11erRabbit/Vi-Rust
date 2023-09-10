
pub mod base;
pub mod prompt;
pub mod info;



use std::{io, collections::HashMap};

use crossterm::event::KeyEvent;

use crate::{pane::{Pane, PaneContainer}, settings::Keys, window::StyledChar};




pub trait Mode {

    fn get_name(&self) -> String;

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut dyn Pane, container: &mut PaneContainer) -> io::Result<bool>;

    fn change_mode(&mut self, name: &str, pane: &mut dyn Pane, container: &mut PaneContainer);

    fn update_status(&mut self, pane: &dyn Pane, container: &PaneContainer) -> (String, String, String);

    fn add_keybindings(&mut self, bindings: HashMap<Keys, String>);

    fn set_key_timeout(&mut self, timeout: u64);

    fn flush_key_buffer(&mut self);

    fn execute_command(&mut self, command: &str, pane: &mut dyn Pane, pane: &mut PaneContainer);

    fn refresh(&mut self);
}

pub trait Promptable: Mode {
    fn draw_prompt(&mut self, row: usize, container: &PaneContainer, output: &mut Vec<Option<StyledChar>>);

    fn max_width(&self) -> usize;
}
