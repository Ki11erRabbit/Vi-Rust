use std::{io, collections::HashMap, rc::Rc, cell::RefCell, time::Instant};

use crossterm::{event::{KeyEvent, KeyCode, KeyModifiers}, execute, cursor::SetCursorStyle};

use crate::{window::Pane, cursor::{Direction, CursorMove, self}, settings::{Keys, Key}};




pub trait Mode {

    fn get_name(&self) -> String;

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut Pane) -> io::Result<bool>;

    fn change_mode(&mut self, name: &str, pane: &mut Pane);

    fn update_status(&self, pane: &Pane) -> (String, String);

    fn add_keybindings(&mut self, bindings: HashMap<Keys, String>);

    fn set_key_timeout(&mut self, timeout: u64);

    fn flush_key_buffer(&mut self);

    fn execute_command(&mut self, command: &str, pane: &mut Pane);

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

    fn execute_command(&mut self, command: &str, pane: &mut Pane) {
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
            "insert_before" => {
                execute!(io::stdout(),SetCursorStyle::BlinkingBar).unwrap();
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

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut Pane) -> io::Result<bool> {
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
            KeyEvent {
                code: KeyCode::Char('0'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.number_buffer.push('0');
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

    fn change_mode(&mut self, name: &str, pane: &mut Pane) {

        pane.set_mode(name);

    }

    fn update_status(&self, pane: &Pane) -> (String, String) {
        let (row, col) = pane.cursor.borrow().get_cursor();
        let mut first = format!("Normal {}:{}", col + 1, row + 1);
        if !self.number_buffer.is_empty() {
            first.push_str(&format!(" {}", self.number_buffer));
        }
        
        let mut second = String::new();
        if !self.key_buffer.is_empty() {
            for key in &self.key_buffer {
                second.push_str(&format!("{}", key));
                second.push_str(" ");
            }
        }

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

    fn move_cursor(&self, direction: KeyCode, pane: &mut Pane) -> io::Result<bool> {
        let cursor = &mut pane.cursor.borrow_mut();
        let direction = match direction {
            KeyCode::Up => Direction::Up,
            KeyCode::Down => Direction::Down,
            KeyCode::Left => Direction::Left,
            KeyCode::Right => Direction::Right,
            _ => return Ok(false),
        };

        cursor.move_cursor(direction, 1, pane.borrow_buffer());
        Ok(true)
    }

    fn insert_newline(&self, pane: &mut Pane) -> io::Result<bool> {
        pane.insert_newline();
        Ok(true)
    }
    fn delete_char(&self, pane: &mut Pane) -> io::Result<bool> {
        pane.delete_char();
        Ok(true)
    }
    fn backspace(&self, pane: &mut Pane) -> io::Result<bool> {
        pane.backspace();
        Ok(true)
    }
    fn insert_char(&self, pane: &mut Pane, c: char) -> io::Result<bool> {
        pane.insert_char(c);
        let cursor = &mut pane.cursor.borrow_mut();
        cursor.move_cursor(Direction::Right, 1, pane.borrow_buffer());
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

    fn execute_command(&mut self, command: &str, pane: &mut Pane) {
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
            "leave" => {
                execute!(io::stdout(),SetCursorStyle::BlinkingBlock).unwrap();
                self.change_mode("Normal", pane);
            },
            command => {
                pane.run_command(command);
            }

        }
    }
    
    fn process_keypress(&mut self, key: KeyEvent, pane: &mut Pane) -> io::Result<bool> {
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

    fn change_mode(&mut self, name: &str, pane: &mut Pane) {
            
        pane.set_mode(name);
    
    }

    fn update_status(&self, pane: &Pane) -> (String, String) {
        let (row, col) = pane.cursor.borrow().get_cursor();
        let first = format!("Insert {}:{}", col + 1, row + 1);

        let mut second = String::new();

        second.push_str(format!("{:?}", pane.cursor.borrow()).as_str());

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

    fn update_status(&self, _pane: &Pane) -> (String, String) {
        let first = format!(":{}", self.command);
        let second = String::new();

        (first, second)
    }

    fn change_mode(&mut self, name: &str, pane: &mut Pane) {
        self.command.clear();
        self.edit_pos = 0;
        pane.set_mode(name);
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


    fn execute_command(&mut self, command: &str, pane: &mut Pane) {
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

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut Pane) -> io::Result<bool> {
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
