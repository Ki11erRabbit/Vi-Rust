use std::io;

use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};

use crate::{window::Pane, cursor::Direction};




pub trait Mode {

    fn process_keypress(&mut self, key: KeyEvent, pane: &mut Pane) -> io::Result<bool>;

}



pub struct Normal {
    name: String,
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
            _ => Ok(false),
            

        }
    }

}
