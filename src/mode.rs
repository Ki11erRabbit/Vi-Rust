use std::{io, collections::HashMap, rc::Rc, cell::RefCell, time::Instant};

use crossterm::{event::{KeyEvent, KeyCode, KeyModifiers}, execute, cursor::SetCursorStyle, style::{Stylize, StyledContent}};

use crate::{window::{Pane, PaneContainer}, cursor::{Direction, CursorMove, self}, settings::{Keys, Key}};




pub trait Mode {

    fn get_name(&self) -> String;

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut dyn Pane) -> io::Result<bool>;

    fn change_mode(&mut self, name: &str, pane: &mut dyn Pane);

    fn update_status(&self, pane: &PaneContainer) -> (String, String);

    fn add_keybindings(&mut self, bindings: HashMap<Keys, String>);

    fn set_key_timeout(&mut self, timeout: u64);

    fn flush_key_buffer(&mut self);

    fn execute_command(&mut self, command: &str, pane: &mut dyn Pane);

    fn refresh(&mut self);

}



pub struct Normal {
    number_buffer: String,
    keybindings: Rc<RefCell<HashMap<Keys, String>>>,
    key_buffer: Vec<Key>,
    timeout: u64,
    time: Instant,
}

impl Normal {
    pub fn new() -> Self {

        Self {
            number_buffer: String::new(),
            keybindings: Rc::new(RefCell::new(HashMap::new())),
            key_buffer: Vec::new(),
            timeout: 1000,
            time: Instant::now(),
        }
    }
}

impl Mode for Normal {

    fn get_name(&self) -> String {
        String::from("Normal")
    }

    fn add_keybindings(&mut self, bindings: HashMap<Keys, String>) {
        self.keybindings.borrow_mut().extend(bindings);
    }

    fn set_key_timeout(&mut self, timeout: u64) {
        self.timeout = timeout;
    }

    fn flush_key_buffer(&mut self) {
        self.key_buffer.clear();
    }

    fn refresh(&mut self) {
        if self.time.elapsed().as_millis() >= self.timeout as u128 {
            self.flush_key_buffer();
            self.time = Instant::now();
        }
    }

    fn execute_command(&mut self, command: &str, pane: &mut dyn Pane) {
        match command {
            "left" => {
                pane.run_command(&format!("move left {}", self.number_buffer));
                self.number_buffer.clear();
            },
            "right" => {
                pane.run_command(&format!("move right {}", self.number_buffer));
                self.number_buffer.clear();
            },
            "up" => {
                pane.run_command(&format!("move up {}", self.number_buffer));
                self.number_buffer.clear();
            },
            "down" => {
                pane.run_command(&format!("move down {}", self.number_buffer));
                self.number_buffer.clear();
            },
            "line_start" => {
                pane.run_command("move line_start");
            },
            "line_end" => {
                pane.run_command("move line_end");
            },
            "file_top" => {
                pane.run_command("move file_top");
            },
            "file_bottom" => {
                pane.run_command("move file_bottom");
            },
            "page_up" => {
                pane.run_command(&format!("move page_up {}", self.number_buffer));
                self.number_buffer.clear();
            },
            "page_down" => {
                pane.run_command(&format!("move page_down {}", self.number_buffer));
                self.number_buffer.clear();
            },
            "insert_before" => {
                execute!(io::stdout(),SetCursorStyle::BlinkingBar).unwrap();
                self.change_mode("Insert", pane);
            },
            "insert_after" => {
                execute!(io::stdout(),SetCursorStyle::BlinkingBar).unwrap();
                pane.run_command("move right 1");
                self.change_mode("Insert", pane);
            },
            "insert_beginning" => {
                execute!(io::stdout(),SetCursorStyle::BlinkingBar).unwrap();
                pane.run_command("move line_start");
                self.change_mode("Insert", pane);
            },
            "insert_end" => {
                execute!(io::stdout(),SetCursorStyle::BlinkingBar).unwrap();
                pane.run_command("move line_end");
                self.change_mode("Insert", pane);
            },
            "start_command" => {
                self.change_mode("Command", pane);
            },
            command => {
                pane.run_command(command);
            }

        }

    }

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut dyn Pane) -> io::Result<bool> {
        self.refresh();

        match key {
            KeyEvent {
                code: KeyCode::Char('1'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.number_buffer.push('1');
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('2'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.number_buffer.push('2');
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('3'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.number_buffer.push('3');
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('4'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.number_buffer.push('4');
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('5'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.number_buffer.push('5');
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('6'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.number_buffer.push('6');
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('7'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.number_buffer.push('7');
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('8'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.number_buffer.push('8');
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('9'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.number_buffer.push('9');
                Ok(true)
            },
            key_event => {
                let key = Key::from(key_event);

                if key.key == KeyCode::Char('0') && !self.number_buffer.is_empty() {
                    self.number_buffer.push('0');
                    return Ok(true);
                }

                let mut flush = false;
                if key.key == KeyCode::Esc {
                    flush = true;
                }
                self.key_buffer.push(key);
                if let Some(command) = self.keybindings.clone().borrow().get(&self.key_buffer) {
                    self.execute_command(command.as_str(), pane);
                    flush = true;
                }
                if flush {
                    self.flush_key_buffer();
                }

                Ok(true)
            }
        }
    }

    fn change_mode(&mut self, name: &str, pane: &mut dyn Pane) {

        pane.change_mode(name);

    }

    fn update_status(&self, pane: &PaneContainer) -> (String, String){
        let (row, col) = pane.get_cursor().borrow().get_cursor();
        let mut first = String::from("Normal");


        let coords = format!("{}:{}", col + 1, row + 1);

        first.push_str(&format!(" {}", coords));
            
        if !self.number_buffer.is_empty() {
            first.push_str(&format!(" {}", self.number_buffer));
        }
        
        let mut second = String::new();
        if !self.key_buffer.is_empty() {
            for key in &self.key_buffer {
                second.push_str(&format!("{} ", key));
            }
        }
        let corners = pane.get_corners();

        let width = corners.1.0 - corners.0.0;
        let height = corners.1.1 - corners.0.1;

        second.push_str(&format!("{:?} ({}, {})", pane.get_corners(), width, height));

        (first, second)
    }

}


pub struct Insert {
    keybindings: Rc<RefCell<HashMap<Keys, String>>>,
    key_buffer: Vec<Key>,
    timeout: u64,
    time: Instant,
}

impl Insert {
    pub fn new() -> Self {
        Self {
            keybindings: Rc::new(RefCell::new(HashMap::new())),
            key_buffer: Vec::new(),
            timeout: 1000,
            time: Instant::now(),
        }
    }

    fn move_cursor(&self, direction: KeyCode, pane: &mut dyn Pane) -> io::Result<bool> {
        let cursor = pane.get_cursor();
        let mut cursor = cursor.borrow_mut();
        let direction = match direction {
            KeyCode::Up => Direction::Up,
            KeyCode::Down => Direction::Down,
            KeyCode::Left => Direction::Left,
            KeyCode::Right => Direction::Right,
            _ => return Ok(false),
        };

        cursor.move_cursor(direction, 1, pane);
        Ok(true)
    }

    fn insert_newline(&self, pane: &mut dyn Pane) -> io::Result<bool> {
        pane.insert_newline();
        Ok(true)
    }
    fn delete_char(&self, pane: &mut dyn Pane) -> io::Result<bool> {
        pane.delete_char();
        Ok(true)
    }
    fn backspace(&self, pane: &mut dyn Pane) -> io::Result<bool> {
        pane.backspace_char();
        Ok(true)
    }
    fn insert_char(&self, pane: &mut dyn Pane, c: char) -> io::Result<bool> {
        pane.insert_char(c);
        let cursor = pane.get_cursor();
        let mut cursor = cursor.borrow_mut();
        cursor.move_cursor(Direction::Right, 1, pane);
        Ok(true)
    }
}

impl Mode for Insert {

    fn get_name(&self) -> String {
        "Insert".to_string()
    }

    fn add_keybindings(&mut self, keybindings: HashMap<Keys, String>) {
        self.keybindings.borrow_mut().extend(keybindings);
    }

    fn set_key_timeout(&mut self, timeout: u64) {
        self.timeout = timeout;
    }

    fn flush_key_buffer(&mut self) {
        self.key_buffer.clear();
    }

    fn refresh(&mut self) {
        if self.time.elapsed().as_millis() >= self.timeout as u128 {
            self.flush_key_buffer();
            self.time = Instant::now();
        }
    }

    fn execute_command(&mut self, command: &str, pane: &mut dyn Pane) {
        match command {
            "left" => {
                pane.run_command("move left 1");
            },
            "right" => {
                pane.run_command("move right 1");
            },
            "up" => {
                pane.run_command("move up 1");
            },
            "down" => {
                pane.run_command("move down 1");
            },
            "file_top" => {
                pane.run_command("move file_top");
            },
            "file_bottom" => {
                pane.run_command("move file_bottom");
            },
            "page_up" => {
                pane.run_command("move page_up");
            },
            "page_down" => {
                pane.run_command("move page_down");
            },
            "leave" => {
                execute!(io::stdout(),SetCursorStyle::BlinkingBlock).unwrap();
                pane.run_command("move left 1");
                self.change_mode("Normal", pane);
            },
            command => {
                pane.run_command(command);
            }

        }
    }
    
    fn process_keypress(&mut self, key: KeyEvent, pane: &mut dyn Pane) -> io::Result<bool> {
        self.refresh();
        
        match key {
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.insert_newline(pane),
            KeyEvent {
                code: KeyCode::Delete,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.delete_char(pane),
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.backspace(pane),
            KeyEvent {
                code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                ..
            } => self.insert_char(pane, match code {
                KeyCode::Char(c) => c,
                KeyCode::Tab => '\t',
                _ => unreachable!(),
            }),
            key_event => {
                let key = Key::from(key_event);

                let mut flush = false;
                if key.key == KeyCode::Esc {
                    flush = true;
                }
                self.key_buffer.push(key);
                if let Some(command) = self.keybindings.clone().borrow().get(&self.key_buffer) {
                    self.execute_command(command.as_str(), pane);
                    flush = true;
                }
                if flush {
                    self.flush_key_buffer();
                }

                Ok(true)
            }
        }

    }

    fn change_mode(&mut self, name: &str, pane: &mut dyn Pane) {
            
        pane.change_mode(name);
    
    }

    fn update_status(&self, pane: &PaneContainer) -> (String, String) {
        let (row, col) = pane.get_cursor().borrow().get_cursor();
        let mut first = String::from("Insert");

        let coords = format!("{}:{}", col + 1, row + 1);
        first.push_str(&format!(" {}", coords));

        let mut second = String::new();

        //second.push_str(&format!("{:?} {}", &pane.borrow_buffer().chars().collect::<String>(), pane.borrow_buffer().line_len()));
        second.push_str(&format!("{:?}", pane.get_cursor().borrow()));

        (first, second)
    }

}


pub struct Command {
    command: String,
    edit_pos: usize,
    keybindings: Rc<RefCell<HashMap<Keys, String>>>,
    key_buffer: Vec<Key>,
    timeout: u64,
    time: Instant,
}

impl Command {
    pub fn new() -> Self {
        Self {
            command: String::new(),
            edit_pos: 0,
            keybindings: Rc::new(RefCell::new(HashMap::new())),
            key_buffer: Vec::new(),
            timeout: 1000,
            time: Instant::now(),
        }
    }
}

impl Mode for Command {

    fn get_name(&self) -> String {
        "Command".to_string()
    }

    fn update_status(&self, pane: &PaneContainer) -> (String, String) {

        let first = format!(":{}", self.command);
        
        let second = String::new();

        (first, second)
    }

    fn change_mode(&mut self, name: &str, pane: &mut dyn Pane) {
        self.command.clear();
        self.edit_pos = 0;
        pane.change_mode(name);
    }

    fn add_keybindings(&mut self, keybindings: HashMap<Keys, String>) {
        self.keybindings.borrow_mut().extend(keybindings);
    }

    fn set_key_timeout(&mut self, timeout: u64) {
        self.timeout = timeout;
    }

    fn flush_key_buffer(&mut self) {
        self.key_buffer.clear();
    }

    fn refresh(&mut self) {
        if self.time.elapsed().as_millis() >= self.timeout as u128 {
            self.flush_key_buffer();
            self.time = Instant::now();
        }
    }


    fn execute_command(&mut self, command: &str, pane: &mut dyn Pane) {
        match command {
            "left" => {
                self.edit_pos = self.edit_pos.saturating_sub(1);
            },
            "right" => {
                if self.edit_pos < self.command.len() {
                    self.edit_pos += 1;
                }
            },
            "start" => {
                self.edit_pos = 0;
            },
            "end" => {
                self.edit_pos = self.command.len();
            },
            "leave" => {
                self.change_mode("Normal", pane);
            },
            command => {
                pane.run_command(command);
            }

        }
    }

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut dyn Pane) -> io::Result<bool> {
        self.refresh();

        match key {
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                pane.run_command(&self.command);

                self.change_mode("Normal", pane);
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Delete,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                if self.edit_pos < self.command.len() {
                    self.command.remove(self.edit_pos);
                }
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                if self.edit_pos > 0 {
                    self.edit_pos -= 1;
                    self.command.remove(self.edit_pos);
                }
                Ok(true)
            },
            KeyEvent {
                code: code @ KeyCode::Char(..),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                ..
            } => {
                let c = match code {
                    KeyCode::Char(c) => c,
                    _ => unreachable!(),
                };


                self.command.insert(self.edit_pos, c);
                self.edit_pos += 1;
                Ok(true)
            },
            key_event => {
                let key = Key::from(key_event);

                let mut flush = false;
                if key.key == KeyCode::Esc {
                    flush = true;
                }
                self.key_buffer.push(key);
                if let Some(command) = self.keybindings.clone().borrow().get(&self.key_buffer) {
                    self.execute_command(command.as_str(), pane);
                    flush = true;
                }
                if flush {
                    self.flush_key_buffer();
                }

                Ok(true)
            }

        }

    }

}
