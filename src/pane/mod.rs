
pub(crate) mod text;
pub mod popup;

use std::{rc::Rc, cell::RefCell, path::PathBuf, io, cmp};

use crossterm::event::KeyEvent;

use crate::{settings::Settings, window::{WindowContents, StyledChar}, cursor::Cursor};


pub enum PaneMessage {
    String(String),
}


impl Clone for PaneContainer {
    fn clone(&self) -> Self {
        Self {
            pane: self.pane.clone(),
            duplicate: true,
            max_size: self.max_size,
            size: self.size,
            position: self.position,
            settings: self.settings.clone(),
            close: false,
        }
    }
}

pub struct PaneContainer {
    pub pane: Rc<RefCell<dyn Pane>>,
    duplicate: bool,
    max_size: (usize, usize),
    size: (usize, usize),
    position: (usize, usize),
    pub settings: Rc<RefCell<Settings>>,
    close: bool,
}

impl PaneContainer {
    pub fn new(max_size: (usize, usize), size: (usize, usize), pane: Rc<RefCell<dyn Pane>>, settings: Rc<RefCell<Settings>>) -> Self {
        let mut container = Self {
            pane,
            duplicate: false,
            max_size,
            size,
            position: (0, 0),
            settings,
            close: false,
        };

        container.shrink();
        container
    }
    fn shrink(&mut self) {
        let (_, (mut end_x, _)) = self.get_corners();
        while  end_x > self.max_size.0 {
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

    pub fn get_pane(&self) -> Rc<RefCell<dyn Pane>> {
        self.pane.clone()
    }

    pub fn is_duplicate(&self) -> bool {
        self.duplicate
    }

    pub fn change_pane(&mut self, pane: Rc<RefCell<dyn Pane>>) {
        self.pane = pane;
        self.duplicate = false;
    }

    pub fn get_filename(&self) -> Option<PathBuf> {
        let pane = self.pane.borrow();
        pane.get_filename().clone()
    }

    pub fn open_file(&mut self, filename: &PathBuf) -> io::Result<()> {
        self.pane.borrow_mut().open_file(filename)
    }

    pub fn scroll_cursor(&mut self) {
        self.pane.borrow_mut().scroll_cursor(self);
    }


    pub fn combine(&mut self, corners: ((usize, usize), (usize, usize))) -> bool {
        eprintln!("Combine: {:?}", corners);
        let ((other_start_x, other_start_y), (other_end_x, other_end_y)) = corners;
        eprintln!("Combine: {:?}", self.get_corners());
        let ((start_x, start_y), (end_x, end_y)) = self.get_corners();


        if other_start_y == start_y || other_end_y == end_y {

            
            //Try combining from the left to right
            if end_x + 1 == other_start_x && start_y == other_start_y && end_y == other_end_y {
                let width = other_end_x - start_x;
                let height = end_y - start_y;
                eprintln!("Width: {}, Height: {}", width, height);

                self.size.0 = width;
                self.size.1 = height;

                self.shrink();
                
                return true;
            }

            //Try combining from the right to left
            else if other_start_x - 1 == end_x && start_y == other_start_y && end_y == other_end_y {
                let width = end_x - other_start_x;
                let height = end_y - start_y;
                eprintln!("Width: {}, Height: {}", width, height);

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
                eprintln!("Width: {}, Height: {}", width, height);

                self.size.0 = width;
                self.size.1 = height;

                self.shrink();

                return true;
            }

            //Try combining from the bottom to top
            else if other_start_y - 1 == end_y && start_x == other_start_x && end_x == other_end_x {
                let width = end_x - start_x;
                let height = end_y - other_start_y;
                eprintln!("Width: {}, Height: {}", width, height);

                self.size.0 = width;
                self.size.1 = height;

                self.shrink();

                return true;
            }
        }

        return false;
    }
    
    pub fn get_size(&self) -> (usize, usize) {
        self.size
    }

    pub fn set_size(&mut self, size: (usize, usize)) {
        self.size = size;
    }


    pub fn resize(&mut self, size: (usize, usize)) {
        //eprintln!("Old Max Size: {:?}", self.max_size);
        //eprintln!("Old Size: {:?}", self.size);
        //eprintln!("New Max Size: {:?}", size);

        let ((start_x, start_y), (end_x, end_y)) = self.get_corners();

        //let new_width = (size.0 * self.size.0) as f64 / self.max_size.0 as f64;
        //let new_height = (size.1 * self.size.1) as f64 / self.max_size.1 as f64;


        //self.size.0 = new_width.ceil() as usize;
        //self.size.1 = new_height.ceil() as usize;

        let new_start_x = (size.0 * start_x) as f64 / self.max_size.0 as f64;
        let new_start_y = (size.1 * start_y) as f64 / self.max_size.1 as f64;

        let new_end_x = (size.0 * end_x) as f64 / self.max_size.0 as f64;
        let new_end_y = (size.1 * end_y) as f64 / self.max_size.1 as f64;

        let new_width = if self.position.0 == 0 {
            cmp::max((new_end_x - new_start_x) as usize, self.settings.borrow().editor_settings.minimum_width)
        }
        else {
            (new_end_x - new_start_x) as usize
        };
        let new_height = if self.position.1 == 0 {
            cmp::max((new_end_y - new_start_y) as usize, self.settings.borrow().editor_settings.minimum_height)
        }
        else {
            (new_end_y - new_start_y) as usize
        };

        self.position.0 = new_start_x as usize;
        self.position.1 = new_start_y as usize;

        self.size.0 = new_width;
        self.size.1 = new_height;

        self.max_size = size;

        self.pane.borrow_mut().resize_cursor(self.size);

        self.shrink();
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

    pub fn get_status(&self) -> (String, String, String) {
        self.pane.borrow().get_status(self)
    }

    pub fn refresh(&mut self) {
        self.pane.borrow_mut().refresh();
    }

    pub fn draw_row(&self, index: usize, contents: &mut Vec<Option<StyledChar>>) {
        self.pane.borrow().draw_row(index, self, contents);
    }

    pub fn process_keypress(&mut self, key: KeyEvent) -> io::Result<bool> {
        self.pane.borrow_mut().process_keypress(key)
    }

    pub fn get_cursor(&self) -> Rc<RefCell<Cursor>> {
        self.pane.borrow().get_cursor()
    }

    pub fn close(&mut self) {
        self.close = true;
    }

    pub fn can_close(&self) -> bool {
        self.close
    }


}

pub trait Pane {
    fn draw_row(&self, index: usize, container: &PaneContainer, contents: &mut Vec<Option<StyledChar>>);

    fn refresh(&mut self);

    fn save_buffer(&mut self) -> io::Result<()>;
    fn open_file(&mut self, filename: &PathBuf) -> io::Result<()>;


    fn process_keypress(&mut self, key: KeyEvent) -> io::Result<bool>;

    fn scroll_cursor(&mut self, container: &PaneContainer);

    fn get_status(&self, container: &PaneContainer) -> (String, String, String);

    fn run_command(&mut self, command: &str);

    fn change_mode(&mut self, mode_name: &str);

    fn insert_newline(&mut self) {
        self.insert_char('\n');
    }
    fn insert_char(&mut self, c: char);
    fn insert_str(&mut self, s: &str);
    fn delete_char(&mut self);
    fn backspace_char(&mut self);

    fn get_cursor(&self) -> Rc<RefCell<Cursor>>;

    fn get_line_count(&self) -> usize;

    fn buffer_to_string(&self) -> String;
    
    fn get_row_len(&self, row: usize) -> Option<usize>;

    fn get_filename(&self) -> &Option<PathBuf>;

    fn resize_cursor(&mut self, size: (usize, usize));
    fn set_cursor_size(&mut self, size: (usize, usize));
}
