use std::io;

use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};

use crate::{window::Pane, cursor::Direction};




pub trait Mode {

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut Pane) -> io::Result<bool>;

    fn change_mode(&mut self, name: &str, pane: &mut Pane);

}



pub struct Normal {
    pub name: String,
    //cmd_buff_update: Box<dyn FnMut(Result<char, &str>) -> ()>,
    number_buffer: String,
}

impl Normal {
    pub fn new(/*cmd_buff_update: Box<dyn FnMut(Result<char, &str>) -> ()>*/) -> Self {
        Self {
            name: String::from("Normal"),
            //cmd_buff_update,
            number_buffer: String::new(),
        }
    }
}

impl Mode for Normal {

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
            _ => Ok(false),
            

        }
    }

    fn change_mode(&mut self, name: &str, pane: &mut Pane) {

        pane.set_mode(name);

    }

}


pub struct Insert {
    pub name: String,
}

impl Insert {
    pub fn new() -> Self {
        Self {
            name: String::from("Insert"),
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
        let cursor = &mut pane.cursor.borrow_mut();
        cursor.move_cursor(Direction::Down, 1, pane.borrow_buffer());
        Ok(true)
    }
    fn delete_char(&self, pane: &mut Pane) -> io::Result<bool> {
        pane.delete_char();
        Ok(true)
    }
    fn backspace(&self, pane: &mut Pane) -> io::Result<bool> {
        pane.backspace();
        let cursor = &mut pane.cursor.borrow_mut();
        cursor.move_cursor(Direction::Left, 1, pane.borrow_buffer());
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

}


