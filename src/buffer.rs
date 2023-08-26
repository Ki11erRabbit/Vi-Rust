use std::{ops::RangeBounds, str::Chars};

use crop::{Rope, RopeSlice};





pub trait Buffer: BufferBase + std::fmt::Display {

    fn new() -> Box<Self> where Self: Sized;

    fn from_text(text: &str) -> Box<Self> where Self: Sized {
        let mut buffer = Self::new();
        (*buffer).insert(0, text);
        buffer
    }

    fn len(&self) -> usize;

    fn line_count(&self) -> usize;
    fn byte_count(&self) -> usize;
    fn line_len(&self, line: usize) -> Option<usize>;
    fn byte_len(&self) -> usize;
    fn get_line(&self, line: usize) -> Option<String>;
    fn get_byte(&self, byte_pos: usize) -> Option<u8>;
    //fn get_line_mut(&mut self, line: usize) -> Option<&mut dyn BufferLine>;
    fn lines(&self) -> &dyn Iterator<Item=&dyn BufferLine>;
    //fn lines_mut(&mut self) -> &mut dyn Iterator<Item=&mut dyn BufferLine>;

    fn line_start_byte(&self, line: usize) -> Option<usize>;
    fn undo(&mut self) -> bool;
    fn redo(&mut self) -> bool;

    fn is_char_boundary(&self, byte_pos: usize) -> bool;
    fn is_empty(&self) -> bool {
        self.byte_count() == 0
    }

    fn is_line_empty(&self, line: usize) -> bool {
        self.line_len(line).unwrap_or(0) == 0
    }

    fn insert(&mut self, byte_pos: usize, text: &str);
    fn replace(&mut self, byte_start: usize, byte_end: usize, text: &str);
    fn delete(&mut self, byte_start: usize, byte_end: usize);


}

pub trait BufferBase {}



pub trait BufferLine {

    fn byte_count(&self) -> usize;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn is_char_boundary(&self, byte_pos: usize) -> bool;

    fn get_string(&self) -> String;
}


pub struct RopeBuffer {
    content: Rope,
    prev: Option<Rope>,
    next: Option<Rope>,
}

impl std::fmt::Display for RopeBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.content)
    }
}

impl BufferBase for RopeBuffer {
}
impl Buffer for RopeBuffer {

    fn new() -> Box<Self> {
        Box::new(Self {
            content: Rope::from(""),
            prev: None,
            next: None,
        })
    }

    fn from_text(text: &str) -> Box<Self> {
        Box::new(Self {
            content: Rope::from(text),
            prev: None,
            next: None,
        })
    }

    fn len(&self) -> usize {
        self.content.chars().count()
    }


    fn insert(&mut self, byte_pos: usize, text: &str) {
        self.prev = Some(self.content.clone());
        self.content.insert(byte_pos, &text);
    }

    fn replace(&mut self, byte_start: usize, byte_end: usize, text: &str) {
        self.prev = Some(self.content.clone());
        self.content.replace(byte_start..byte_end, &text);
    }

    fn delete(&mut self, byte_start: usize, byte_end: usize) {
        self.prev = Some(self.content.clone());
        self.content.delete(byte_start..byte_end);
    }
    
    fn line_count(&self) -> usize {
        let line_count = self.content.line_len();
        if let Some('\n') = self.content.chars().last() {
            line_count + 1
        } else {
            line_count
        }
    }

    fn byte_count(&self) -> usize {
        self.content.byte_len()
    }

    fn line_len(&self, line: usize) -> Option<usize> {
        if self.content.line_len() < line {
            return None;
        }
        let line_len = self.content.line(line).chars().count();
        Some(line_len)
    }

    fn byte_len(&self) -> usize {
        self.content.byte_len()
    }

    fn get_line(&self, line: usize) -> Option<String> {
        if self.content.line_len() < line {
            return None;
        }
        Some(self.content.line(line).to_string())
    }

    fn get_byte(&self, byte_pos: usize) -> Option<u8> {
        if self.content.byte_len() < byte_pos {
            return None;
        }
        Some(self.content.byte(byte_pos))
    }


    fn lines<'input>(&'input self) -> &'input dyn Iterator<Item=&dyn BufferLine> {
        unimplemented!()
    }


    fn line_start_byte(&self, line: usize) -> Option<usize> {
        if self.content.line_len() < line {
            return None;
        }
        let line_start_byte = self.content.byte_of_line(line);
        Some(line_start_byte)
    }


    fn undo(&mut self) -> bool {
        if let Some(prev) = self.prev.take() {
            self.next = Some(self.content.clone());
            self.content = prev;
            true
        } else {
            false
        }
    }

    fn redo(&mut self) -> bool {
        if let Some(next) = self.next.take() {
            self.prev = Some(self.content.clone());
            self.content = next;
            true
        } else {
            false
        }
    }


    fn is_char_boundary(&self, byte_pos: usize) -> bool {
        self.content.is_char_boundary(byte_pos)
    }

    

}

impl BufferBase for Box<RopeBuffer> {
}

impl Buffer for Box<RopeBuffer> {
    fn new() -> Box<Self> {
        Box::new(RopeBuffer::new())
    }

    fn from_text(text: &str) -> Box<Self> {
        Box::new(RopeBuffer::from_text(text))
    }

    fn len(&self) -> usize {
        self.content.chars().count()
    }


    fn insert(&mut self, byte_pos: usize, text: &str) {
        self.prev = Some(self.content.clone());
        self.content.insert(byte_pos, &text);
    }

    fn replace(&mut self, byte_start: usize, byte_end: usize, text: &str) {
        self.prev = Some(self.content.clone());
        self.content.replace(byte_start..byte_end, &text);
    }

    fn delete(&mut self, byte_start: usize, byte_end: usize) {
        self.prev = Some(self.content.clone());
        self.content.delete(byte_start..byte_end);
    }
    
    fn line_count(&self) -> usize {
        let line_count = self.content.line_len();
        if let Some('\n') = self.content.chars().last() {
            line_count + 1
        } else {
            line_count
        }
    }

    fn byte_count(&self) -> usize {
        self.content.byte_len()
    }

    fn line_len(&self, line: usize) -> Option<usize> {
        if self.content.line_len() < line {
            return None;
        }
        let line_len = self.content.line(line).chars().count();
        Some(line_len)
    }

    fn byte_len(&self) -> usize {
        self.content.byte_len()
    }

    fn get_line(&self, line: usize) -> Option<String> {
        if self.content.line_len() < line {
            return None;
        }
        Some(self.content.line(line).to_string())
    }


    fn get_byte(&self, byte_pos: usize) -> Option<u8> {
        if self.content.byte_len() < byte_pos {
            return None;
        }
        Some(self.content.byte(byte_pos))
    }

    fn lines<'input>(&'input self) -> &'input dyn Iterator<Item=&dyn BufferLine> {
        unimplemented!()
    }


    fn line_start_byte(&self, line: usize) -> Option<usize> {
        if self.content.line_len() < line {
            return None;
        }
        let line_start_byte = self.content.byte_of_line(line);
        Some(line_start_byte)
    }


    fn undo(&mut self) -> bool {
        if let Some(prev) = self.prev.take() {
            self.next = Some(self.content.clone());
            self.content = prev;
            true
        } else {
            false
        }
    }

    fn redo(&mut self) -> bool {
        if let Some(next) = self.next.take() {
            self.prev = Some(self.content.clone());
            self.content = next;
            true
        } else {
            false
        }
    }


    fn is_char_boundary(&self, byte_pos: usize) -> bool {
        self.content.is_char_boundary(byte_pos)
    }
    

}



impl BufferLine for RopeSlice<'static> {
    fn byte_count(&self) -> usize {
        self.byte_len()
    }

    fn len(&self) -> usize {
        self.chars().count()
    }

    fn is_char_boundary(&self, byte_pos: usize) -> bool {
        self.is_char_boundary(byte_pos)
    }

    fn get_string(&self) -> String {
        self.chars().collect::<String>()
    }
}
    








