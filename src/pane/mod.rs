
//pub(crate) mod text;
//pub mod popup;
//pub mod treesitter;
pub mod text_pane;


use std::{rc::Rc, cell::RefCell, path::PathBuf, io, cmp, fmt::Debug, sync::mpsc::Sender};

use crossterm::event::KeyEvent;
use uuid::Uuid;

use crate::{settings::Settings, window::{StyledChar, TextRow, WindowMessage}, cursor::Cursor, buffer::Buffer};


pub enum PaneMessage {
    String(String),
    Close,
}


impl Clone for PaneContainer {
    fn clone(&self) -> Self {
        Self {
            pane: self.pane.clone(),
            duplicate: true,
            max_size: self.max_size,
            size: self.size,
            original_size: self.original_size,
            position: self.position,
            settings: self.settings.clone(),
            close: false,
            identifier: Uuid::new_v4(),
            move_not_resize: self.move_not_resize,
        }
    }
}

impl Debug for PaneContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(PaneContainer)")
    }
}


pub struct PaneContainer {
    pub pane: Rc<RefCell<dyn Pane>>,
    duplicate: bool,
    max_size: (usize, usize),
    size: (usize, usize),
    original_size: (usize, usize),
    position: (usize, usize),
    settings: Rc<RefCell<Settings>>,
    close: bool,
    identifier: Uuid,
    move_not_resize: bool,
}

impl PaneContainer {
    pub fn new(pane: Rc<RefCell<dyn Pane>>, settings: Rc<RefCell<Settings>>) -> Self {

        let size = settings.borrow().get_window_size();
        
        let mut container = Self {
            pane,
            duplicate: false,
            max_size: size,
            original_size: size,
            size,
            position: (0, 0),
            settings,
            close: false,
            identifier: Uuid::new_v4(),
            move_not_resize: false,
        };

        container.shrink();
        container
    }

    pub fn changed(&mut self) {
        self.pane.borrow_mut().changed();
    }


    pub fn reset(&mut self) {
        self.pane.borrow_mut().reset();
    }


    pub fn change_pane(&mut self, pane: Rc<RefCell<dyn Pane>>) {
        self.pane = pane;
        self.duplicate = false;
    }

    pub fn get_name(&self) -> String {
        let pane = self.pane.borrow();
        pane.get_name().to_string()
    }

    pub fn get_status(&self) -> Option<(String, String, String)> {
        let pane = self.pane.clone();
        pane.borrow().get_status(self)
    }


    pub fn draw_row(&self, index: usize, contents: &mut TextRow) {
        self.pane.borrow().draw_row(index, self, contents);
    }

    pub fn process_keypress(&mut self, key: KeyEvent) {
        let pane = self.pane.clone();
        let mut pane = pane.borrow_mut();
        pane.process_keypress(key, self);
    }

    pub fn get_cursor_location(&self) -> Option<(usize, usize)> {
        self.pane.borrow().get_cursor()
    }

    pub fn close(&mut self) {
        self.close = true;
    }

    pub fn can_close(&self) -> bool {
        self.close
    }

    pub fn backup(&mut self) {
        self.pane.borrow_mut().backup();
    }
    
    pub fn refresh(&mut self) {
        let pane = self.pane.clone();
        let mut pane = pane.borrow_mut();
        pane.refresh(self);
    }

    pub fn set_location(&mut self, position: (usize, usize)) {
        self.pane.borrow_mut().set_location(position);
    }


    pub fn combine(&mut self, corners: ((usize, usize), (usize, usize))) -> bool {
        //eprintln!("Combine: {:?}", corners);
        let ((other_start_x, other_start_y), (other_end_x, other_end_y)) = corners;
        //eprintln!("Combine: {:?}", self.get_corners());
        let ((start_x, start_y), (end_x, end_y)) = self.get_corners();


        if other_start_y == start_y || other_end_y == end_y {
            
            //Try combining from the left to right
            if end_x + 1 == other_start_x && start_y == other_start_y && end_y == other_end_y {
                let width = other_end_x - start_x;
                let height = end_y - start_y;
                //eprintln!("Width: {}, Height: {}", width, height);

                self.size.0 = width;
                self.size.1 = height;

                self.shrink();
                
                return true;
            }

            //Try combining from the right to left
            else if other_start_x.saturating_sub(1) == end_x && start_y == other_start_y && end_y == other_end_y {
                let width = end_x - other_start_x;
                let height = end_y - start_y;
                //eprintln!("Width: {}, Height: {}", width, height);

                self.size.0 = width;
                self.size.1 = height;

                self.shrink();

                return true;
            }
        }
        else if other_start_x == start_x || other_end_x == end_x {

            //Try combining from the top to bottom
            if end_y + 1 == other_start_y && start_x == other_start_x && end_x == other_end_x {
                let width = end_x - start_x;
                let height = other_end_y - start_y;
                //eprintln!("Width: {}, Height: {}", width, height);

                self.size.0 = width;
                self.size.1 = height;

                self.shrink();

                return true;
            }

            //Try combining from the bottom to top
            else if other_start_y - 1 == end_y && start_x == other_start_x && end_x == other_end_x {
                let width = end_x - start_x;
                let height = end_y - other_start_y;
                //eprintln!("Width: {}, Height: {}", width, height);

                self.size.0 = width;
                self.size.1 = height;

                self.shrink();

                return true;
            }
        }

        return false;
    }

    pub fn resize(&mut self, max_size: (usize, usize)) {

        //eprintln!("Old Size: {:?}", self.size);
        //eprintln!("Max Size: {:?}", self.max_size);
        //eprintln!("New Max Size: {:?}", max_size);

        self.pane.borrow_mut().changed();
        //eprintln!("Old Max Size: {:?}", self.max_size);
        //eprintln!("Old Size: {:?}", self.size);
        //eprintln!("New Max Size: {:?}", size);

        let ((start_x, start_y), (end_x, end_y)) = self.get_corners();

        //let new_width = (size.0 * self.size.0) as f64 / self.max_size.0 as f64;
        //let new_height = (size.1 * self.size.1) as f64 / self.max_size.1 as f64;


        //self.size.0 = new_width.ceil() as usize;
        //self.size.1 = new_height.ceil() as usize;

        let new_start_x = (max_size.0 * start_x) as f64 / self.max_size.0 as f64;
        let new_start_y = (max_size.1 * start_y) as f64 / self.max_size.1 as f64;

        let new_end_x = (max_size.0 * end_x) as f64 / self.max_size.0 as f64;
        let new_end_y = (max_size.1 * end_y) as f64 / self.max_size.1 as f64;

        let new_width = if self.position.0 == 0 {
            //new_start_x += 1.0;
            
            cmp::max((new_end_x - new_start_x) as usize, self.settings.borrow().editor_settings.minimum_width)
        }
        else {
            //new_start_x += 1.0;
            (new_end_x - new_start_x) as usize
        };
        let new_height = if self.position.1 == 0 {
            //new_start_y += 1.0;
            cmp::max((new_end_y - new_start_y) as usize, self.settings.borrow().editor_settings.minimum_height)
        }
        else {
            //new_start_y += 1.0;
            (new_end_y - new_start_y) as usize
        };

        self.position.0 = new_start_x as usize;
        self.position.1 = new_start_y as usize;

        //eprintln!("New Position: {:?}", self.position);

        if !self.move_not_resize {
            self.size.0 = new_width;
            self.size.1 = new_height - 1;

            self.max_size = max_size;

            self.grow();
            self.shrink();
        }
        self.pane.borrow_mut().resize(self.size);

        //eprintln!("New Size: {:?}", self.size);
        
    }

}

/// The getters and setters for the pane container
impl PaneContainer {

    pub fn set_move_not_resize(&mut self, move_not_resize: bool) {
        self.move_not_resize = move_not_resize;
    }

    pub fn get_uuid(&self) -> Uuid {
        self.identifier
    }

    pub fn get_pane(&self) -> Rc<RefCell<dyn Pane>> {
        self.pane.clone()
    }

    pub fn is_duplicate(&self) -> bool {
        self.duplicate
    }
    pub fn get_size(&self) -> (usize, usize) {
        self.size
    }

    pub fn set_size(&mut self, size: (usize, usize)) {
        self.size = size;
    }


    pub fn set_position(&mut self, position: (usize, usize)) {
        self.position = position;
        self.shrink();
    }

    pub fn get_position(&self) -> (usize, usize) {
        self.position
    }

    pub fn get_corners(&self) -> ((usize, usize), (usize, usize)) {
        let x = self.size.0 + self.position.0;
        let y = self.size.1 + self.position.1;
        (self.position, (x, y))
    }

}


/// The utility functions for PaneContainer
impl PaneContainer {

    fn shrink(&mut self) {
        //eprintln!("Max Size: {:?}", self.max_size);
        //eprintln!("Before Shrink: {:?}", self.get_corners());
        
        let (_, (mut end_x, _)) = self.get_corners();
        while end_x > self.max_size.0 {
            if self.size.0 == 0 {
                break;
            }
            self.size.0 = self.size.0.saturating_sub(1);
            (_, (end_x, _)) = self.get_corners();
        }
        let (_, (_, mut end_y)) = self.get_corners();
        while  end_y > self.max_size.1 {
            if self.size.1 == 0 {
                break;
            }
            self.size.1 = self.size.1.saturating_sub(1);
            (_, (_, end_y)) = self.get_corners();
        }

        //eprintln!("After Shrink: {:?}", self.get_corners());

    }

    fn grow(&mut self) {
        //eprintln!("Max Size: {:?}", self.max_size);
        //eprintln!("Before Shrink: {:?}", self.get_corners());
        
        let (_, (mut end_x, _)) = self.get_corners();
        while end_x < self.max_size.0 {
            if self.size.0 == self.max_size.0 {
                break;
            }
            self.size.0 = self.size.0.saturating_add(1);
            (_, (end_x, _)) = self.get_corners();
        }
        let (_, (_, mut end_y)) = self.get_corners();
        while  end_y < self.max_size.1 - 1 {
            if self.size.1 == self.max_size.1 - 1 {
                break;
            }
            self.size.1 = self.size.1.saturating_add(1);
            (_, (_, end_y)) = self.get_corners();
        }

        //eprintln!("After Shrink: {:?}", self.get_corners());

    }

}


/*pub trait Pane {
    fn draw_row(&self, index: usize, container: &PaneContainer, contents: &mut TextRow);

    fn refresh(&mut self, container: &mut PaneContainer);


    fn process_keypress(&mut self, key: KeyEvent, container: &mut PaneContainer) -> io::Result<bool>;

    fn scroll_cursor(&mut self, container: &PaneContainer);

    fn get_status(&self, container: &PaneContainer) -> (String, String, String);

    fn run_command(&mut self, command: &str, container: &PaneContainer);

    /// The difference bettween run_command and this function is that this function
    /// will try to execute the command in the current mode, and if it fails it will
    /// try to execute it in the pane.
    fn execute_command(&mut self, command: &str, container: &mut PaneContainer);

    fn change_mode(&mut self, mode_name: &str);


    fn get_settings(&self) -> Rc<RefCell<Settings>>;

    fn set_sender(&mut self, sender: Sender<WindowMessage>);

    /// This function is called after we redraw the screen
    /// For most panes it should tell the cursor that it hasn't moved yet
    fn reset(&mut self);

    /// This gets called whenever we do an action that would cause a redraw of the screen.
    fn changed(&mut self);

    fn get_cursor(&self) -> Rc<RefCell<Cursor>>;

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

    fn get_filename(&self) -> &Option<PathBuf>;

    fn resize_cursor(&mut self, size: (usize, usize));
    fn set_cursor_size(&mut self, size: (usize, usize));
    fn borrow_buffer(&self) -> &Buffer;
    fn borrow_mut_buffer(&mut self) -> &mut Buffer;
    
}*/


pub trait Pane {
    fn draw_row(&self, index: usize, container: &PaneContainer, contents: &mut TextRow);

    fn refresh(&mut self, container: &mut PaneContainer);

    fn process_keypress(&mut self, key: KeyEvent, container: &mut PaneContainer) -> io::Result<bool>;

    /// This function gets the status bar for the pane
    /// It returns the name of the mode, a first item, and a second item
    /// The option is to tell the window that the pane doesn't have a status bar and
    /// it should draw the main pane on layer 0
    fn get_status(&self, container: &PaneContainer) -> Option<(String, String, String)>;

    /// This function is called after we redraw the screen
    /// For most panes it should tell the cursor that it hasn't moved yet
    fn reset(&mut self);

    /// This gets called whenever we do an action that would cause a redraw of the screen.
    fn changed(&mut self);

    fn get_cursor(&self) -> Option<(usize, usize)>;

    fn get_name(&self) -> &str;

    fn run_command(&mut self, command: &str, container: &mut PaneContainer);

    fn resize(&mut self, size: (usize, usize));

    fn set_location(&mut self, location: (usize, usize));

    fn get_settings(&self) -> Rc<RefCell<Settings>>;

    fn change_mode(&mut self, mode: &str);

    fn backup(&mut self);
}


pub trait TextBuffer: Pane {
    
    fn save_buffer(&mut self) -> io::Result<()>;
    fn open_file(&mut self, filename: PathBuf) -> io::Result<()>;
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

    fn get_physical_cursor(&self) -> Rc<RefCell<Cursor>>;

}
