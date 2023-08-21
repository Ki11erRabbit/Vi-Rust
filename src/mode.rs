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
                code: KeyCode::Char('h'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let num = if let Ok(num) = self.number_buffer.parse::<usize>() {
                    num
                } else {
                    1
                };

                let cursor = &mut pane.cursor.borrow_mut();
                cursor.move_cursor(Direction::Left, num, pane.borrow_buffer());
                
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let num = if let Ok(num) = self.number_buffer.parse::<usize>() {
                    num
                } else {
                    1
                };

                let cursor = &mut pane.cursor.borrow_mut();
                cursor.move_cursor(Direction::Down, num, pane.borrow_buffer());
                
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let num = if let Ok(num) = self.number_buffer.parse::<usize>() {
                    num
                } else {
                    1
                };

                let cursor = &mut pane.cursor.borrow_mut();
                cursor.move_cursor(Direction::Up, num, pane.borrow_buffer());
                
                Ok(true)
            },
            KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::NONE,
                ..
            } => {
                let num = if let Ok(num) = self.number_buffer.parse::<usize>() {
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
