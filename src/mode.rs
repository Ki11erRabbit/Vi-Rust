use std::io;

use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};

use crate::{window::Pane, cursor::{Direction, CursorMove}};




pub trait Mode {

    fn get_name(&self) -> String;

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut Pane) -> io::Result<bool>;

    fn change_mode(&mut self, name: &str, pane: &mut Pane);

    fn update_status(&self, pane: &Pane) -> String;

}



pub struct Normal {
    number_buffer: String,
}

impl Normal {
    pub fn new() -> Self {
        Self {
            number_buffer: String::new(),
        }
    }
}

impl Mode for Normal {

    fn get_name(&self) -> String {
        String::from("Normal")
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
            KeyEvent {
                code: KeyCode::Char('h') | KeyCode::Left,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let num = if let Ok(num) = self.number_buffer.parse::<usize>() {
                    self.number_buffer.clear();
                    num
                } else {
                    1
                };

                let cursor = &mut pane.cursor.borrow_mut();
                cursor.move_cursor(Direction::Left, num, pane.borrow_buffer());
                
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('j') | KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let num = if let Ok(num) = self.number_buffer.parse::<usize>() {
                    self.number_buffer.clear();
                    num
                } else {
                    1
                };

                let cursor = &mut pane.cursor.borrow_mut();
                cursor.move_cursor(Direction::Down, num, pane.borrow_buffer());
                
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('k') | KeyCode::Up,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let num = if let Ok(num) = self.number_buffer.parse::<usize>() {
                    self.number_buffer.clear();
                    num
                } else {
                    1
                };

                let cursor = &mut pane.cursor.borrow_mut();
                cursor.move_cursor(Direction::Up, num, pane.borrow_buffer());
                
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('l') | KeyCode::Right,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let num = if let Ok(num) = self.number_buffer.parse::<usize>() {
                    self.number_buffer.clear();
                    num
                } else {
                    1
                };

                let cursor = &mut pane.cursor.borrow_mut();
                cursor.move_cursor(Direction::Right, num, pane.borrow_buffer());
                
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
            _ => Ok(true),
            

        }
    }

    fn change_mode(&mut self, name: &str, pane: &mut Pane) {

        pane.set_mode(name);

    }

    fn update_status(&self, pane: &Pane) -> String {
        let (row, col) = pane.cursor.borrow().get_cursor();
        format!("Normal {}:{}", row, col)
    }

}


pub struct Insert;

impl Insert {
    pub fn new() -> Self {
        Self {}
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
    
    fn process_keypress(&mut self, key: KeyEvent, pane: &mut Pane) -> io::Result<bool> {

        match key {
            KeyEvent {
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
            } => self.move_cursor(direction, pane),
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
            KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.change_mode("Normal", pane);
                Ok(true)
            },
            _ => Ok(false),

        }

    }

    fn change_mode(&mut self, name: &str, pane: &mut Pane) {
            
        pane.set_mode(name);
    
    }

    fn update_status(&self, pane: &Pane) -> String {
        let (row, col) = pane.cursor.borrow().get_cursor();
        format!("Insert {}:{}   {:?} {}", row, col, pane.borrow_buffer().to_string().as_str(), pane.borrow_buffer().lines().count())
    }

}


pub struct Command {
    command: String,
    edit_pos: usize,
}

impl Command {
    pub fn new() -> Self {
        Self {
            command: String::new(),
            edit_pos: 0,
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

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut Pane) -> io::Result<bool> {

        match key {
            KeyEvent {
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
            },
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
            KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                self.change_mode("Normal", pane);
                Ok(true)
            },
            _ => Ok(false),

        }

    }

}
