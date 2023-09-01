use std::{cmp, cell::RefCell, rc::Rc};

use crop::{Rope, RopeSlice};

use crate::settings::Settings;





pub struct Buffer {
    current: usize,
    buffers: Vec<Rope>,
    settings: Rc<RefCell<Settings>>,
}


impl Buffer {
    pub fn new(settings: Rc<RefCell<Settings>>) -> Self {
        Self {
            current: 0,
            buffers: vec![Rope::new()],
            settings,
        }
    }

    pub fn set_settings(&mut self, settings: Rc<RefCell<Settings>>) {
        self.settings = settings;
    }

    pub fn undo(&mut self) {
        if self.current > 0 {
            self.current -= 1;
        }
    }

    pub fn redo(&mut self) {
        if self.current < self.buffers.len() - 1 {
            self.current += 1;
        }
    }

    pub fn line_len(&self, row: usize) -> Option<usize> {
        self.buffers[self.current].lines().nth(row).map(|line| line.chars().map(|c| if c == '\t' {
            self.settings.borrow().editor_settings.tab_size
        } else {
            1
        }).sum())
    }

    pub fn get_line_count(&self) -> usize {
        let mut num_lines = self.buffers[self.current].line_len();
        if let Some('\n') = self.buffers[self.current].chars().last() {
            num_lines += 1;
        }
        num_lines
    }

    pub fn get_char_count(&self) -> usize {
        self.buffers[self.current].chars().count()
    }

    pub fn get_byte_count(&self) -> usize {
        self.buffers[self.current].bytes().count()
    }

    pub fn get_row(&self, row: usize, offset: usize, col: usize) -> Option<RopeSlice> {
        if row >= self.buffers[self.current].line_len() {
            return None;
        }
        let line = self.buffers[self.current].line(row);
        let len = cmp::min(col + offset, line.line_len().saturating_sub(offset));
        if len == 0 {
            return None;
        }
        Some(line.line_slice(offset..len))
    }

    pub fn get_byte_offset(&self, x: usize, y: usize) -> Option<usize> {
        if y >= self.buffers[self.current].line_len() {
            return None;
        }
        let line_byte = self.buffers[self.current].byte_of_line(y);

        let line = self.buffers[self.current].line(y);
        let mut i = 0;
        let mut col_byte = 0;
        while i < line.byte_len() {
            if line.is_char_boundary(i) {
                if col_byte == x {
                    break;
                }
                col_byte += 1;
            }
            i += 1;
        }
        Some(line_byte + col_byte)
    }

    fn get_new_rope(&mut self) -> &mut Rope {
        let buffer = self.buffers[self.current].clone();
        if self.current < self.buffers.len() - 1 {
            self.buffers.truncate(self.current + 1);
        }
        self.buffers.push(buffer);
        self.current += 1;
        &mut self.buffers[self.current]
    }

    pub fn add_new_rope(&mut self) {
        if self.current > 0 {
            if self.buffers[self.current - 1] == self.buffers[self.current] {
                return;
            }
        }
        let buffer = self.buffers[self.current].clone();
        if self.current < self.buffers.len() - 1 {
            self.buffers.truncate(self.current + 1);
        }
        self.buffers.push(buffer);
        self.current += 1;
    }

    pub fn insert_current<T>(&mut self, byte_offset: usize, text: T) where T: AsRef<str> {
        self.buffers[self.current].insert(byte_offset, text.as_ref());
    }

    pub fn delete_current<R>(&mut self, range: R) where R: std::ops::RangeBounds<usize> {
        self.buffers[self.current].delete(range);
    }

    pub fn replace_current<R, T>(&mut self, range: R, text: T) where R: std::ops::RangeBounds<usize>, T: AsRef<str> {
        self.buffers[self.current].replace(range, text.as_ref());
    }

    pub fn insert<T>(&mut self, byte_offset: usize, text: T) where T: AsRef<str> {
        let buffer = self.get_new_rope();
        buffer.insert(byte_offset, text.as_ref());
    }

    pub fn delete<R>(&mut self, range: R) where R: std::ops::RangeBounds<usize> {
        let buffer = self.get_new_rope();
        buffer.delete(range);
    }

    pub fn replace<R, T>(&mut self, range: R, text: T) where R: std::ops::RangeBounds<usize>, T: AsRef<str> {
        let buffer = self.get_new_rope();
        buffer.replace(range, text.as_ref());
    }

    pub fn get_nth_byte(&self, n: usize) -> Option<u8> {
        self.buffers[self.current].bytes().nth(n)
    }

    pub fn get_nth_char(&self, n: usize) -> Option<char> {
        self.buffers[self.current].chars().nth(n)
    }

    pub fn insert_chain<T>(&mut self, values: Vec<(usize, T)>)
        where T: AsRef<str>
    {
        let buffer = self.get_new_rope();
        for (offset, text) in values.iter().rev() {
            buffer.insert(*offset, text.as_ref());
        }
    }

    pub fn delete_chain<R>(&mut self, values: Box<[R]>)
        where R: std::ops::RangeBounds<usize> + Copy
    {
        let buffer = self.get_new_rope();
        for range in values.iter().rev() {
            buffer.delete(*range);
        }
    }

    pub fn replace_chain<R, T>(&mut self, values: Box<[(R, T)]>)
        where R: std::ops::RangeBounds<usize> + Copy, T: AsRef<str>
    {
        let buffer = self.get_new_rope();
        for (range, text) in values.iter().rev() {
            buffer.replace(*range, text.as_ref());
        }
    }

}

impl std::fmt::Display for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.buffers[self.current])
    }
}


impl From<&str> for Buffer {
    fn from(s: &str) -> Self {
        Self {
            current: 0,
            buffers: vec![Rope::from(s)],
            settings: Rc::new(RefCell::new(Settings::default())),
        }
    }
}

impl From<String> for Buffer {
    fn from(s: String) -> Self {
        Self {
            current: 0,
            buffers: vec![Rope::from(s)],
            settings: Rc::new(RefCell::new(Settings::default())),
        }
    }
}

impl From<&String> for Buffer {
    fn from(s: &String) -> Self {
        Self {
            current: 0,
            buffers: vec![Rope::from(s.as_str())],
            settings: Rc::new(RefCell::new(Settings::default())),
        }
    }
}
