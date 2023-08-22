use std::{io, collections::HashMap, rc::Rc, cell::RefCell};

use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};

use crate::{window::Pane, cursor::{Direction, CursorMove}, settings::{Keys, Key}};




pub trait Mode {

    fn get_name(&self) -> String;

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut Pane) -> io::Result<bool>;

    fn change_mode(&mut self, name: &str, pane: &mut Pane);

    fn update_status(&self, pane: &Pane) -> String;

    fn add_keybindings(&mut self, bindings: HashMap<Keys, String>);

    fn flush_key_buffer(&mut self);

    fn execute_command(&mut self, command: &str, pane: &mut Pane);

}



pub struct Normal {
    number_buffer: String,
    keybindings: Rc<RefCell<HashMap<Keys, String>>>,
    key_buffer: Vec<Key>,
}

impl Normal {
    pub fn new() -> Self {
        Self {
            number_buffer: String::new(),
            keybindings: Rc::new(RefCell::new(HashMap::new())),
            key_buffer: Vec::new(),
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

    fn flush_key_buffer(&mut self) {
        self.key_buffer.clear();
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
                self.change_mode("Insert", pane);
            },
            "start_command" => {
                self.change_mode("Command", pane);
            },
            _ => unreachable!(),

        }

    }

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut Pane) -> io::Result<bool> {

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

                if key.key == KeyCode::Null {
                    self.flush_key_buffer();
                }
                else {
                    self.key_buffer.push(key);
                    if let Some(command) = self.keybindings.clone().borrow().get(&self.key_buffer) {
                        self.execute_command(command.as_str(), pane);
                    }
                    self.flush_key_buffer();
                }

                Ok(true)
            }
            /*KeyEvent {
                code: KeyCode::Char('h') | KeyCode::Left,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                pane.run_command(&format!("move left {}", self.number_buffer));
                self.number_buffer.clear();
                
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('j') | KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                pane.run_command(&format!("move down {}", self.number_buffer));
                self.number_buffer.clear();
                
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('k') | KeyCode::Up,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                pane.run_command(&format!("move up {}", self.number_buffer));
                self.number_buffer.clear();
                
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('l') | KeyCode::Right,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                pane.run_command(&format!("move right {}", self.number_buffer));
                self.number_buffer.clear();
                
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('i'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.change_mode("Insert", pane);
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char(':'),
                //modifiers: KeyModifiers::SHIFT,
                ..
            } => {
                self.change_mode("Command", pane);
                Ok(true)
            },
            _ => Ok(true),*/
            

        }
    }

    fn change_mode(&mut self, name: &str, pane: &mut Pane) {

        pane.set_mode(name);

    }

    fn update_status(&self, pane: &Pane) -> String {
        let (row, col) = pane.cursor.borrow().get_cursor();
        format!("Normal {}:{}", col, row)
    }

}


pub struct Insert {
    keybindings: Rc<RefCell<HashMap<Keys, String>>>,
    key_buffer: Vec<Key>,
}

impl Insert {
    pub fn new() -> Self {
        Self {
            keybindings: Rc::new(RefCell::new(HashMap::new())),
            key_buffer: Vec::new(),
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

    fn flush_key_buffer(&mut self) {
        self.key_buffer.clear();
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
                self.change_mode("Normal", pane);
            },
            _ => unreachable!(),

        }
    }
    
    fn process_keypress(&mut self, key: KeyEvent, pane: &mut Pane) -> io::Result<bool> {

        match key {
            /*KeyEvent {
                code:
                direction
                    @
                    (KeyCode::Up
                     | KeyCode::Down
                     | KeyCode::Left
                     | KeyCode::Right
                    ),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.move_cursor(direction, pane),*/
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

                if key.key == KeyCode::Null {
                    self.flush_key_buffer();
                }
                else {
                    self.key_buffer.push(key);
                    if let Some(command) = self.keybindings.clone().borrow().get(&self.key_buffer) {
                        self.execute_command(command.as_str(), pane);
                    }
                    self.flush_key_buffer();
                }

                Ok(true)
            }
            /*KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.change_mode("Normal", pane);
                Ok(true)
            },
            _ => Ok(false),*/

        }

    }

    fn change_mode(&mut self, name: &str, pane: &mut Pane) {
            
        pane.set_mode(name);
    
    }

    fn update_status(&self, pane: &Pane) -> String {
        let (row, col) = pane.cursor.borrow().get_cursor();
        format!("Insert {}:{}", col, row)
    }

}


pub struct Command {
    command: String,
    edit_pos: usize,
    keybindings: Rc<RefCell<HashMap<Keys, String>>>,
    key_buffer: Vec<Key>,
}

impl Command {
    pub fn new() -> Self {
        Self {
            command: String::new(),
            edit_pos: 0,
            keybindings: Rc::new(RefCell::new(HashMap::new())),
            key_buffer: Vec::new(),
        }
    }
}

impl Mode for Command {

    fn get_name(&self) -> String {
        "Command".to_string()
    }

    fn update_status(&self, _pane: &Pane) -> String {
        format!(":{}", self.command)
    }

    fn change_mode(&mut self, name: &str, pane: &mut Pane) {
        self.command.clear();
        self.edit_pos = 0;
        pane.set_mode(name);
    }

    fn add_keybindings(&mut self, keybindings: HashMap<Keys, String>) {
        self.keybindings.borrow_mut().extend(keybindings);
    }

    fn flush_key_buffer(&mut self) {
        self.key_buffer.clear();
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
            _ => unreachable!(),

        }
    }

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut Pane) -> io::Result<bool> {

        match key {
            /*KeyEvent {
                code:
                direction
                    @
                    (KeyCode::Up
                     | KeyCode::Down
                     | KeyCode::Left
                     | KeyCode::Right
                    ),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                match direction {
                    KeyCode::Left => {
                        self.edit_pos = self.edit_pos.saturating_sub(1);
                    },
                    KeyCode::Right => {
                        if self.edit_pos < self.command.len() {
                            self.edit_pos += 1;
                        }
                    },
                    KeyCode::Up => {
                        self.edit_pos = self.command.len();
                    },
                    KeyCode::Down => {
                        self.edit_pos = 0;
                    },
                    _ => unreachable!(),
                }
                Ok(true)
            },*/
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
            /*KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.change_mode("Normal", pane);
                Ok(true)
            },*/
            key_event => {
                let key = Key::from(key_event);

                if key.key == KeyCode::Null {
                    if let Some(command) = self.keybindings.clone().borrow().get(&self.key_buffer) {
                        self.execute_command(command.as_str(), pane);
                    }
                    self.flush_key_buffer();
                }
                else {
                    self.key_buffer.push(key);
                }

                Ok(true)
            }

        }

    }

}
