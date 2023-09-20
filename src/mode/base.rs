use std::{io, collections::HashMap, rc::Rc, cell::RefCell, time::Instant};

use crossterm::{event::{KeyEvent, KeyCode, KeyModifiers}, execute, cursor::{SetCursorStyle, MoveTo}, terminal};

use crate::{pane::{Pane, PaneContainer, TextBuffer}, cursor::{Direction, Cursor}, settings::{Keys, Key}};

use crate::mode::Mode;

use super::TextMode;


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



}

impl TextMode for Normal {

    fn change_mode(&mut self, name: &str, pane: &mut dyn TextBuffer, _container: &mut PaneContainer) {

        pane.change_mode(name);

    }

    fn execute_command(&mut self, command: &str, pane: &mut dyn TextBuffer, container: &mut PaneContainer) {
        let mut command_args = command.split_whitespace();
        let command = command_args.next().unwrap_or("");
        
        match command {
            "left" => {
                pane.run_command(&format!("move left {}", self.number_buffer), container);
                self.number_buffer.clear();
            },
            "right" => {
                pane.run_command(&format!("move right {}", self.number_buffer), container);
                self.number_buffer.clear();
            },
            "up" => {
                pane.run_command(&format!("move up {}", self.number_buffer), container);
                self.number_buffer.clear();
            },
            "down" => {
                pane.run_command(&format!("move down {}", self.number_buffer), container);
                self.number_buffer.clear();
            },
            "line_start" => {
                pane.run_command("move line_start", container);
            },
            "line_end" => {
                pane.run_command("move line_end", container);
            },
            "file_top" => {
                pane.run_command("move file_top", container);
            },
            "file_bottom" => {
                pane.run_command("move file_bottom", container);
            },
            "page_up" => {
                pane.run_command(&format!("move page_up {}", self.number_buffer), container);
                self.number_buffer.clear();
            },
            "page_down" => {
                pane.run_command(&format!("move page_down {}", self.number_buffer), container);
                self.number_buffer.clear();
            },
            "insert_before" => {
                execute!(io::stdout(),SetCursorStyle::BlinkingBar).unwrap();
                self.change_mode("Insert", pane, container);
            },
            "insert_after" => {
                execute!(io::stdout(),SetCursorStyle::BlinkingBar).unwrap();
                pane.run_command("move right 1", container);
                self.change_mode("Insert",pane, container);
            },
            "insert_beginning" => {
                execute!(io::stdout(),SetCursorStyle::BlinkingBar).unwrap();
                pane.run_command("move line_start", container);
                self.change_mode("Insert", pane, container);
            },
            "insert_end" => {
                execute!(io::stdout(),SetCursorStyle::BlinkingBar).unwrap();
                pane.run_command("move line_end", container);
                self.change_mode("Insert", pane, container);
            },
            "start_command" => {
                self.change_mode("Command", pane, container);
            },
            "paste_after" => {
                eprintln!("paste after");
                pane.run_command(&format!("paste {}", self.number_buffer), container);
            },
            "paste_before" => {
                pane.run_command(&format!("paste {}", self.number_buffer), container);
            },
            "insert_text" => {
                let text = command_args.collect::<Vec<&str>>().join(" ");

                pane.insert_str(&text);
                
            },
            "copy_line" => {
                pane.run_command(&format!("copy line {}", self.number_buffer), container);
            },
            command => {
                pane.run_command(command, container);
            }

        }

    }

    fn process_keypress(&mut self, key: Key, pane: &mut dyn TextBuffer, container: &mut PaneContainer) {
        self.refresh();

        match key {
            Key {
                key: KeyCode::Char('1'),
                modifier: KeyModifiers::NONE,
            } => {
                self.number_buffer.push('1');
            },
            Key {
                key: KeyCode::Char('2'),
                modifier: KeyModifiers::NONE,
            } => {
                self.number_buffer.push('2');
            },
            Key {
                key: KeyCode::Char('3'),
                modifier: KeyModifiers::NONE,
            } => {
                self.number_buffer.push('3');
            },
            Key {
                key: KeyCode::Char('4'),
                modifier: KeyModifiers::NONE,
            } => {
                self.number_buffer.push('4');
            },
            Key {
                key: KeyCode::Char('5'),
                modifier: KeyModifiers::NONE,
            } => {
                self.number_buffer.push('5');
            },
            Key {
                key: KeyCode::Char('6'),
                modifier: KeyModifiers::NONE,
            } => {
                self.number_buffer.push('6');
            },
            Key {
                key: KeyCode::Char('7'),
                modifier: KeyModifiers::NONE,
            } => {
                self.number_buffer.push('7');
            },
            Key {
                key: KeyCode::Char('8'),
                modifier: KeyModifiers::NONE,
            } => {
                self.number_buffer.push('8');
            },
            Key {
                key: KeyCode::Char('9'),
                modifier: KeyModifiers::NONE,
            } => {
                self.number_buffer.push('9');
            },
            key => {

                if key.key == KeyCode::Char('0') && !self.number_buffer.is_empty() {
                    self.number_buffer.push('0');
                    return;
                }

                let mut flush = false;
                if key.key == KeyCode::Esc {
                    flush = true;
                }
                self.key_buffer.push(key);
                if let Some(command) = self.keybindings.clone().borrow().get(&self.key_buffer) {
                    self.execute_command(command.as_str(), pane, container);
                    flush = true;
                }
                if flush {
                    self.flush_key_buffer();
                }

            }
        }
    }

    fn update_status(&mut self, pane: &dyn TextBuffer, _container: &PaneContainer) -> (String, String, String){
        let (row, col) = pane.get_physical_cursor().borrow().get_cursor();


        let mut first = format!("{}:{}", col + 1, row + 1);

        
            
        if !self.number_buffer.is_empty() {
            first.push_str(&format!(" {}", self.number_buffer));
        }
        
        let mut second = String::new();
        if !self.key_buffer.is_empty() {
            for key in &self.key_buffer {
                second.push_str(&format!("{} ", key));
            }
        }

        /*let corners = pane.get_corners();

        let width = corners.1.0 - corners.0.0;
        let height = corners.1.1 - corners.0.1;

        second.push_str(&format!("{:?} ({}, {})", pane.get_corners(), width, height));*/

        (self.get_name(), first, second)
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

    /*fn move_cursor(&self, direction: KeyCode, pane: &mut dyn Pane) -> io::Result<bool> {
        let cursor = pane.get_cursor();
        let mut cursor = cursor.borrow_mut();
        let direction = match direction {
            KeyCode::Up => Direction::Up,
            KeyCode::Down => Direction::Down,
            KeyCode::Left => Direction::Left,
            KeyCode::Right => Direction::Right,
            _ => return Ok(false),
        };

        cursor.move_cursor(direction, 1, &mut *pane);
        Ok(true)
    }*/

    fn insert_newline(&self, pane: &mut dyn TextBuffer) {
        pane.insert_newline();
        pane.changed();
    }
    fn delete_char(&self, pane: &mut dyn TextBuffer) {
        pane.delete_char();
        pane.changed();
    }
    fn backspace(&self, pane: &mut dyn TextBuffer)  {
        pane.backspace_char();
        pane.changed();
    }
    fn insert_char(&self, pane: &mut dyn TextBuffer, c: char)  {
        pane.changed();
        if pane.get_settings().borrow().editor_settings.use_spaces && c == '\t' {
            pane.insert_str(&" ".repeat(pane.get_settings().borrow().editor_settings.tab_size));
        } else {
            pane.insert_char(c);
        };
        
        let cursor = pane.get_physical_cursor();
        let mut cursor = cursor.borrow_mut();
        if c == '\t' {
            let tab_size = pane.get_settings().borrow().editor_settings.tab_size;
            cursor.move_cursor(Direction::Right, tab_size, &mut *pane);
        } else {
            cursor.move_cursor(Direction::Right, 1, &mut *pane);
        }
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




}

impl TextMode for Insert {

    fn change_mode(&mut self, name: &str, pane: &mut dyn TextBuffer, container: &mut PaneContainer) {
        pane.change_mode(name);
    
    }

    fn execute_command(&mut self, command: &str, pane: &mut dyn TextBuffer, container: &mut PaneContainer) {
        match command {
            "left" => {
                pane.run_command("move left 1", container);
            },
            "right" => {
                pane.run_command("move right 1", container);
            },
            "up" => {
                pane.run_command("move up 1", container);
            },
            "down" => {
                pane.run_command("move down 1", container);
            },
            "file_top" => {
                pane.run_command("move file_top", container);
            },
            "file_bottom" => {
                pane.run_command("move file_bottom", container);
            },
            "page_up" => {
                pane.run_command("move page_up", container);
            },
            "page_down" => {
                pane.run_command("move page_down", container);
            },
            "leave" => {
                execute!(io::stdout(),SetCursorStyle::BlinkingBlock).unwrap();
                pane.run_command("move left 1", container);
                self.change_mode("Normal", pane, container);
            },
            command => {
                pane.run_command(command, container);
            }

        }
    }
    
    fn process_keypress(&mut self, key: Key, pane: &mut dyn TextBuffer, container: &mut PaneContainer) {
        self.refresh();

        if self.key_buffer.is_empty() {
            match key {
                Key {
                    key: KeyCode::Enter,
                    modifier: KeyModifiers::NONE,
                } => {self.insert_newline(pane);},
                Key {
                    key: KeyCode::Delete,
                    modifier: KeyModifiers::NONE,
                } => {self.delete_char(pane);},
                Key {
                    key: KeyCode::Backspace,
                    modifier: KeyModifiers::NONE,
                } => {self.backspace(pane);},
                Key {
                    key: code @ (KeyCode::Char(..) | KeyCode::Tab),
                    modifier: KeyModifiers::NONE | KeyModifiers::SHIFT,
                } => {self.insert_char(pane, match code {
                    KeyCode::Char(c) => c,
                    KeyCode::Tab => '\t',
                    _ => unreachable!(),
                });},
                key => {
                    let mut flush = false;
                    if key.key == KeyCode::Esc {
                        flush = true;
                    }
                    self.key_buffer.push(key);
                    if let Some(command) = self.keybindings.clone().borrow().get(&self.key_buffer) {
                        self.execute_command(command.as_str(), pane, container);
                        flush = true;
                    }
                    if flush {
                        self.flush_key_buffer();
                    }
                }
            }
        }
        else {

            let mut flush = false;
            if key.key == KeyCode::Esc {
                flush = true;
            }
            self.key_buffer.push(key);
            if let Some(command) = self.keybindings.clone().borrow().get(&self.key_buffer) {
                self.execute_command(command.as_str(), pane, container);
                flush = true;
            }
            if flush {
                self.flush_key_buffer();
            }

        }

    }

    fn update_status(&mut self, pane: &dyn TextBuffer, container: &PaneContainer) -> (String, String, String) {
        let (row, col) = pane.get_physical_cursor().borrow().get_cursor();

        let first = format!("{}:{}", col + 1, row + 1);
        

        let mut second = String::new();

        //second.push_str(&format!("{:?} {}", &pane.borrow_buffer().chars().collect::<String>(), pane.borrow_buffer().line_len()));
        //second.push_str(&format!("{:?}", pane.get_cursor().borrow()));

        if !self.key_buffer.is_empty() {
            for key in &self.key_buffer {
                second.push_str(&format!("{} ", key));
            }
        }


        (self.get_name(), first, second)
    }


}

pub struct Command {
    command: String,
    edit_pos: usize,
    keybindings: Rc<RefCell<HashMap<Keys, String>>>,
    key_buffer: Vec<Key>,
    timeout: u64,
    time: Instant,
    cursor_location: Option<Cursor>,
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
            cursor_location: None,
        }
    }


    fn backup_cursor(&mut self, pane: &dyn TextBuffer) {
        if self.cursor_location.is_none() {
            self.cursor_location = Some(*pane.get_physical_cursor().borrow());
        }
    }

}

impl Mode for Command {

    fn get_name(&self) -> String {
        "Command".to_string()
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


}

impl TextMode for Command {

    fn change_mode(&mut self, name: &str, pane: &mut dyn TextBuffer, container: &mut PaneContainer) {
        self.command.clear();
        self.edit_pos = 0;
        pane.change_mode(name);

        let mut cursor = self.cursor_location.take().unwrap();

        execute!(io::stdout(), SetCursorStyle::BlinkingBlock).unwrap();
        let (x, y) = cursor.get_real_cursor();

        if !pane.get_physical_cursor().borrow().jumped {
            *pane.get_physical_cursor().borrow_mut() = cursor;
        }

        pane.get_physical_cursor().borrow_mut().ignore_offset = false;

        execute!(io::stdout(), MoveTo(x as u16,y as u16)).unwrap();
        
    }


    fn update_status(&mut self, pane: &dyn TextBuffer, _container: &PaneContainer) -> (String, String, String) {


        execute!(io::stdout(),SetCursorStyle::BlinkingBar).unwrap();

        let pane = &*pane;

        self.backup_cursor(pane);
        
        let cursor = pane.get_physical_cursor();
        
        let mut cursor = cursor.borrow_mut();
        cursor.number_line_size = 0;
        cursor.ignore_offset = true;

        let offset = self.get_name().len() + 2;// + 1 for the space and + 1 for the colon

        cursor.set_draw_cursor(offset + self.edit_pos, terminal::size().unwrap().1 as usize);
        
        let first = format!(":{}", self.command);
        
        let second = String::new();
        

        (self.get_name(), first, second)
    }


    fn execute_command(&mut self, command: &str, pane: &mut dyn TextBuffer, container: &mut PaneContainer) {
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
                self.change_mode("Normal", pane, container);
            },
            command => {
                pane.run_command(command, container);

            }

        }
    }

    fn process_keypress(&mut self, key: Key, pane: &mut dyn TextBuffer, container: &mut PaneContainer) {
        self.refresh();


        match key {
            Key {
                key: KeyCode::Enter,
                modifier: KeyModifiers::NONE,
            } => {
                pane.run_command(&self.command, container);

                self.change_mode("Normal", pane,container);
            },
            Key {
                key: KeyCode::Delete,
                modifier: KeyModifiers::NONE,
            } => {
                if self.edit_pos < self.command.len() {
                    self.command.remove(self.edit_pos);
                }
            },
            Key {
                key: KeyCode::Backspace,
                modifier: KeyModifiers::NONE,
            } => {
                if self.edit_pos > 0 {
                    self.edit_pos -= 1;
                    self.command.remove(self.edit_pos);
                }
            },
            Key {
                key: code @ KeyCode::Char(..),
                modifier: KeyModifiers::NONE | KeyModifiers::SHIFT,
            } => {
                let c = match code {
                    KeyCode::Char(c) => c,
                    _ => unreachable!(),
                };


                self.command.insert(self.edit_pos, c);
                self.edit_pos += 1;
            },
            key => {

                let mut flush = false;
                if key.key == KeyCode::Esc {
                    flush = true;
                }
                self.key_buffer.push(key);
                if let Some(command) = self.keybindings.clone().borrow().get(&self.key_buffer) {
                    self.execute_command(command.as_str(), pane, container);
                    flush = true;
                }
                if flush {
                    self.flush_key_buffer();
                }

            }

        }
    }

}
