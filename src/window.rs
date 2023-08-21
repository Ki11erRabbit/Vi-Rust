use std::cell::RefCell;
use std::cmp;
use std::collections::HashMap;
use std::rc::Rc;
use std::io;
use std::io::Write;

use crop::{Rope, RopeSlice};
use crossterm::event::KeyEvent;
use crossterm::{terminal::{self, ClearType}, execute, cursor, queue};

use crate::mode::{Mode, Normal, Insert};
use crate::cursor::Cursor;




const TAB_SIZE: usize = 4;


pub struct Window {
    size: (usize, usize),
    contents: WindowContents,
    pane: Pane,
}

impl Window {
    pub fn new() -> Self {
        let win_size = terminal::size()
            .map(|(w, h)| (w as usize, h as usize - 2))// -1 for trailing newline and -1 for command bar
            .unwrap();
        Self {
            size: win_size,
            contents: WindowContents::new(),
            pane: Pane::new(win_size),
        }
    }

    pub fn run(&mut self) -> io::Result<bool> {
        Ok(true)
    }

    pub fn clear_screen() -> io::Result<()> {
        execute!(std::io::stdout(), terminal::Clear(terminal::ClearType::All))?;
        execute!(std::io::stdout(), cursor::MoveTo(0, 0))
    }

    fn draw_rows(&mut self) {
        let rows = self.size.1;
        let cols = self.size.0;

        for i in 0..rows {
            let real_row = i + self.pane.cursor.borrow().row_offset;

            let offset = 0 + self.pane.cursor.borrow().col_offset;
            if let Some(row) = self.pane.get_row(real_row, offset, cols) {
                row.chars().for_each(|c| if c == '\t' {
                    self.contents.push_str(" ".repeat(TAB_SIZE).as_str());
                } else {
                    self.contents.push(c);
                });

                queue!(
                    self.contents,
                    terminal::Clear(ClearType::UntilNewLine),
                ).unwrap();
            }


            self.contents.push_str("\r\n");

        }

    }


    pub fn draw_status_bar(&mut self) {
        self.contents.push_str("\r\n");
        self.contents.push_str(format!("{:?}", self.pane.cursor.borrow()).as_str());
    }

    pub fn refresh_screen(&mut self) -> io::Result<()> {

        self.pane.scroll_cursor();

        queue!(
            self.contents,
            cursor::Hide,
            cursor::MoveTo(0, 0),
        )?;

        self.draw_rows();
        self.draw_status_bar();

        let (x, y) = self.pane.cursor.borrow().get_real_cursor();

        
        let x = {
            if let Some(row) = self.pane.borrow_buffer().lines().nth(y) {
                let len = row.chars().count();
                cmp::min(x, len)
            }
            else {
                0
            }
        };

        queue!(
            self.contents,
            cursor::MoveTo(x as u16, y as u16),
            cursor::Show,
        )?;

        self.contents.flush()
    }

    pub fn open_file(&mut self, filename: &str) -> io::Result<()> {
        self.pane.open_file(filename)
    }

    pub fn process_keypress(&mut self, key: KeyEvent) -> io::Result<bool> {
        self.pane.process_keypress(key)
    }

}

pub struct WindowContents {
    content: String,
}

impl WindowContents {
    pub fn new() -> Self {
        Self {
            content: String::new(),
        }
    }

    fn push(&mut self, c: char) {
        self.content.push(c);
    }

    fn push_str(&mut self, s: &str) {
        self.content.push_str(s);
    }
}

impl io::Write for WindowContents {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.content.push_str(s);
                Ok(buf.len())
            }
            Err(_) => Err(io::ErrorKind::WriteZero.into()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let out = write!(std::io::stdout(), "{}", self.content);
        std::io::stdout().flush()?;
        self.content.clear();
        out
    }


}

pub struct Pane {
    size: (usize, usize),
    contents: Rope,
    mode: Rc<RefCell<dyn Mode>>,
    modes: HashMap<String, Rc<RefCell<dyn Mode>>>,
    pub cursor: Rc<RefCell<Cursor>>,
}


impl Pane {
    pub fn new(size: (usize, usize)) -> Self {
        let mut modes: HashMap<String, Rc<RefCell<dyn Mode>>> = HashMap::new();
        let normal = Rc::new(RefCell::new(Normal::new()));
        let insert = Rc::new(RefCell::new(Insert::new()));

        modes.insert("Normal".to_string(), normal.clone());
        modes.insert("Insert".to_string(), insert.clone());
        Self {
            size,
            contents: Rope::new(),
            mode: normal,
            modes,
            cursor: Rc::new(RefCell::new(Cursor::new(size))),
        }
    }
    pub fn get_size(&self) -> (usize, usize) {
        self.size
    }

    pub fn get_row(&self, row: usize, offset: usize, col: usize) -> Option<RopeSlice> {
        if row >= self.contents.line_len() {
            return None;
        }
        let line = self.contents.line(row);
        let len = cmp::min(col + offset, line.line_len() - offset);
        Some(line.line_slice(offset..len))
    }

    pub fn open_file(&mut self, filename: &str) -> io::Result<()> {
        let file = std::fs::read_to_string(filename)?;
        self.contents = Rope::from(file);
        Ok(())
    }

    pub fn process_keypress(&mut self, key: KeyEvent) -> io::Result<bool> {
        let mode = self.mode.clone();
        let result = mode.borrow_mut().process_keypress(key, self);
        result
    }

    pub fn borrow_buffer(&self) -> &Rope {
        &self.contents
    }

    pub fn borrow_buffer_mut(&mut self) -> &mut Rope {
        &mut self.contents
    }

    pub fn scroll_cursor(&mut self) {
        let cursor = self.cursor.clone();

        cursor.borrow_mut().scroll(self);
        
    }

    pub fn get_mode(&self, name: &str) -> Option<Rc<RefCell<dyn Mode>>> {
        self.modes.get(name).map(|m| m.clone())
    }

    pub fn set_mode(&mut self, name: &str) {
        if let Some(mode) = self.get_mode(name) {
            self.mode = mode;
        }
    }

    fn get_byte_offset(&self) -> usize {
        let (x, y) = self.cursor.borrow().get_cursor();
        let line_pos = self.contents.byte_of_line(y);
        let row_pos = x;

        let byte_pos = line_pos + row_pos;

        byte_pos
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    ///TODO: add check to make sure we have a valid byte range
    pub fn delete_char(&mut self) {
        let byte_pos = self.get_byte_offset();

        self.contents.delete(byte_pos..byte_pos + 1);
    }

    ///TODO: add check to make sure we have a valid byte range
    pub fn backspace(&mut self) -> bool {
        let byte_pos = self.get_byte_offset();
        let mut ret = false;
        if self.contents.chars().nth(byte_pos).is_some() && self.contents.chars().nth(byte_pos).unwrap() == '\n' {
            ret = true;
        }

        self.contents.delete(byte_pos - 1..byte_pos);
        ret
    }

    pub fn insert_char(&mut self, c: char) {
        let byte_pos = self.get_byte_offset();
        let c = c.to_string();
        self.contents.insert(byte_pos, c);
    }
        
        
}
