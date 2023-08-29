
pub mod base;


use std::{io, collections::HashMap, rc::Rc, cell::RefCell, time::Instant};

use crossterm::{event::{KeyEvent, KeyCode, KeyModifiers}, execute, cursor::{SetCursorStyle, MoveTo}, terminal};

use crate::{pane::{Pane, PaneContainer}, cursor::{Direction, Cursor}, settings::{Keys, Key}};




pub trait Mode {

    fn get_name(&self) -> String;

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut dyn Pane) -> io::Result<bool>;

    fn change_mode(&mut self, name: &str, pane: &mut dyn Pane);

    fn update_status(&mut self, pane: &PaneContainer) -> (String, String, String);

    fn add_keybindings(&mut self, bindings: HashMap<Keys, String>);

    fn set_key_timeout(&mut self, timeout: u64);

    fn flush_key_buffer(&mut self);

    fn execute_command(&mut self, command: &str, pane: &mut dyn Pane);

    fn refresh(&mut self);
}
