use std::{cell::RefCell, rc::Rc, path::PathBuf, io, cmp};

use crossterm::event::KeyEvent;
use uuid::Uuid;

use crate::{settings::Settings, buffer::Buffer, new_editor::LayerRow};



pub mod text_pane;








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
            id: Uuid::new_v4(),
            move_not_resize: self.move_not_resize,
        }

    }

}

pub struct PaneContainer {
    pane: Rc<RefCell<dyn Pane>>,
    duplicate: bool,
    max_size: (usize, usize),
    size: (usize, usize),
    original_size: (usize, usize),
    position: (usize, usize),
    settings: Rc<RefCell<Settings>>,
    close: bool,
    id: Uuid,
    move_not_resize: bool,
}

impl PaneContainer {
    pub fn new(pane: Rc<RefCell<dyn Pane>>, settings: Rc<RefCell<Settings>>) -> Self {
        let size = settings.borrow().get_window_size();

        let position = (0, 0);

        let id = Uuid::new_v4();
        Self {
            pane,
            duplicate: false,
            max_size: size,
            size,
            original_size: size,
            position,
            settings,
            close: false,
            id,
            move_not_resize: false,
        }
    }

    pub fn set_move_not_resize(&mut self, value: bool) {
        self.move_not_resize = value;
    }

    pub fn get_move_not_resize(&self) -> bool {
        self.move_not_resize
    }

    pub fn set_max_size(&mut self, size: (usize, usize)) {
        self.max_size = size;
    }
    
    pub fn set_size(&mut self, size: (usize, usize)) {
        self.size = size;
    }

    pub fn set_position(&mut self, position: (usize, usize)) {
        self.position = position;
    }

    pub fn get_position(&self) -> (usize, usize) {
        self.position
    }

    pub fn get_status(&self) -> (String, String, String) {
        self.pane.borrow().get_status(self)
    }

    pub fn get_settings(&self) -> Rc<RefCell<Settings>> {
        self.settings.clone()
    }

    pub fn get_corners(&self) -> ((usize, usize), (usize, usize)) {
        let (x, y) = self.position;
        let (width, height) = self.size;
        ((x, y), (x + width, y + height))
    }

    pub fn get_id(&self) -> Uuid {
        self.id
    }

    pub fn is_duplicate(&self) -> bool {
        self.duplicate
    }

    pub fn get_name(&self) -> String {
        self.pane.borrow().get_name().to_string()
    }

    pub fn get_size(&self) -> (usize, usize) {
        self.size
    }

    pub fn change_pane(&mut self, pane: Rc<RefCell<dyn Pane>>) {
        self.pane = pane;
        self.changed();
        self.duplicate = false;
    }

    pub fn get_pane(&self) -> Rc<RefCell<dyn Pane>> {
        self.pane.clone()
    }

    pub fn changed(&mut self) {
        self.pane.borrow_mut().changed();
    }

    pub fn reset(&mut self) {
        self.pane.borrow_mut().reset();
    }

    pub fn refresh(&mut self) {
        let pane = self.pane.clone();
        pane.borrow_mut().refresh(self);
    }

    pub fn get_cursor_coords(&self) -> Option<(usize, usize)> {
        self.pane.borrow().get_cursor()
    }

    pub fn draw_row(&mut self, row: usize, contents: &mut LayerRow) {
        let pane = self.pane.clone();
        pane.borrow_mut().draw_row(row, self, contents);
    }

    pub fn process_keypress(&mut self, key: KeyEvent) -> io::Result<()> {
        let pane = self.pane.clone();
        let x = pane.borrow_mut().process_keypress(key, self);
        x
    }

    pub fn draw_status(&self) -> bool {
        self.pane.borrow().draw_status()
    }

    pub fn set_location(&mut self, location: (usize, usize)) {
        self.pane.borrow_mut().set_location(location);
    }

    pub fn close(&mut self) {
        self.close = true;
    }

    pub fn can_close(&self) -> bool {
        self.close
    }

    pub fn lose_focus(&mut self) {
        self.pane.borrow_mut().changed();
    }

    pub fn combine(&mut self, corners: ((usize, usize), (usize, usize))) -> bool {
        let ((other_start_x, other_start_y), (other_end_x, other_end_y)) = corners;
        let ((start_x, start_y), (end_x, end_y)) = self.get_corners();


        if other_start_y == start_y || other_end_y == end_y {
            
            //Try combining from the left to right
            if end_x + 1 == other_start_x && start_y == other_start_y && end_y == other_end_y {
                let width = other_end_x - start_x;
                let height = end_y - start_y;

                self.size.0 = width;
                self.size.1 = height;

                self.shrink();
                
                return true;
            }

            //Try combining from the right to left
            else if other_start_x.saturating_sub(1) == end_x && start_y == other_start_y && end_y == other_end_y {
                let width = end_x - other_start_x;
                let height = end_y - start_y;

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

                self.size.0 = width;
                self.size.1 = height;

                self.shrink();

                return true;
            }

            //Try combining from the bottom to top
            else if other_start_y - 1 == end_y && start_x == other_start_x && end_x == other_end_x {
                let width = end_x - start_x;
                let height = end_y - other_start_y;

                self.size.0 = width;
                self.size.1 = height;

                self.shrink();

                return true;
            }
        }

        return false;
    }


    ///TODO: make this resize relative to the original size
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

/// The private functions for the pane container
impl PaneContainer {

    fn shrink(&mut self) {
        
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
    }

    fn grow(&mut self) {
        
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
    }

    
}


pub trait Pane {
    fn draw_row(&self, row: usize, container: &PaneContainer, output: &mut LayerRow);

    fn refresh(&mut self, container: &mut PaneContainer);

    fn process_keypress(&mut self, key: KeyEvent, container: &mut PaneContainer) -> io::Result<()>;

    /// This function gets the status bar for the pane
    /// It returns the name of the mode, a first item, and a second item
    fn get_status(&self, container: &PaneContainer) -> (String, String, String);

    /// This function returns whether or not the status bar should be drawn
    fn draw_status(&self) -> bool;

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

}
