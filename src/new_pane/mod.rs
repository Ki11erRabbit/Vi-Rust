use std::{cell::RefCell, rc::Rc, path::PathBuf, io};

use crossterm::event::KeyEvent;
use uuid::Uuid;

use crate::{settings::Settings, buffer::Buffer};



pub mod text_pane;








pub enum PaneMessage {
    String(String),
    Close,
}


pub struct PaneContainer {
    pane: Rc<RefCell<dyn Pane>>,
    duplicate: bool,
    size: (usize, usize),
    position: (usize, usize),
    settings: Rc<RefCell<Settings>>,
    close: bool,
    id: Uuid,
    move_not_resize: bool,
}




pub trait Pane {
    fn draw_row(&self, row: usize, container: &PaneContainer, output: &mut TextRow);

    fn refresh(&mut self, container: &mut PaneContainer);

    fn process_keypress(&mut self, key: KeyEvent, container: &mut PaneContainer) -> io::Result<()>;

    /// This function gets the status bar for the pane
    /// It returns the name of the mode, a first item, and a second item
    fn get_status(&self, container: &PaneContainer) -> (String, String, String);

    fn draw_status(&self) -> bool;

    /// This function is called after we redraw the screen
    /// For most panes it should tell the cursor that it hasn't moved yet
    fn reset(&mut self);

    /// This gets called whenever we do an action that would cause a redraw of the screen.
    fn changed(&mut self);

    fn get_cursor(&self) -> Option<(usize, usize)>;

    fn get_name(&self) -> &str;

    fn run_command(&mut self, command: &str, container: &mut PaneContainer);
}


pub trait TextPane: Pane {
    
    fn save_buffer(&mut self) -> io::Result<()>;
    fn open_file(&mut self, filename: &PathBuf) -> io::Result<()>;
    fn backup_buffer(&mut self);

    fn insert_newline(&mut self) {
        self.insert_char('\n');
    }
    fn insert_char(&mut self, c: char);
    fn insert_str(&mut self, s: &str);
    fn delete_char(&mut self);
    fn backspace_char(&mut self);


    fn get_line_count(&self) -> usize;

    fn buffer_to_string(&self) -> String;
    
    fn get_row_len(&self, row: usize) -> Option<usize>;

    fn borrow_buffer(&self) -> &Buffer;
    fn borrow_mut_buffer(&mut self) -> &mut Buffer;

}
