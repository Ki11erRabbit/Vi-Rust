use std::ops::RangeBounds;

use crop::{Rope, RopeSlice};









pub trait Buffer {

    fn line_count(&self) -> usize;
    fn byte_count(&self) -> usize;
    fn line_length(&self, line: usize) -> Option<usize>;
    fn byte_length(&self, line: usize) -> Option<usize>;
    fn get_line(&self, line: usize) -> Option<String>;
    //fn get_line_mut(&mut self, line: usize) -> Option<&mut dyn BufferLine>;
    fn lines(&self) -> &dyn Iterator<Item=&dyn BufferLine>;
    //fn lines_mut(&mut self) -> &mut dyn Iterator<Item=&mut dyn BufferLine>;
    fn insert<T>(&mut self, byte_pos: usize, text: T) where T: AsRef<str>;
    fn replace<R, T>(&mut self, byte_range: R, text: T) where R: RangeBounds<usize>, T: AsRef<str>;
    fn delete<R>(&mut self, byte_range: R) where R: RangeBounds<usize>;
    fn undo(&mut self) -> bool;
    fn redo(&mut self) -> bool;

    fn is_char_boundary(&self, byte_pos: usize) -> bool;
    fn is_empty(&self) -> bool {
        self.byte_count() == 0
    }

}


pub trait BufferLine {

    fn byte_count(&self) -> usize;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn is_char_boundary(&self, byte_pos: usize) -> bool;
}





pub struct RopeBuffer {
    content: Rope,
    prev: Option<Rope>,
    next: Option<Rope>,
}

impl Buffer for RopeBuffer {
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

    fn line_length(&self, line: usize) -> Option<usize> {
        if self.content.line_len() < line {
            return None;
        }
        let line_len = self.content.line(line).chars().count();
        Some(line_len)
    }

    fn byte_length(&self, line: usize) -> Option<usize> {
        if self.content.line_len() < line {
            return None;
        }
        Some(self.content.line(line).byte_len())
    }

    fn get_line(&self, line: usize) -> Option<String> {
        if self.content.line_len() < line {
            return None;
        }
        Some(self.content.line(line).to_string())
    }


    fn lines<'input>(&'input self) -> &'input dyn Iterator<Item=&dyn BufferLine> {
        unimplemented!()
    }


    fn insert<T>(&mut self, byte_pos: usize, text: T) where T: AsRef<str> {
        self.prev = Some(self.content.clone());
        self.content.insert(byte_pos, text);
    }

    fn replace<R, T>(&mut self, byte_range: R, text: T) where R: RangeBounds<usize>, T: AsRef<str> {
        self.prev = Some(self.content.clone());
        self.content.replace(byte_range, text);
    }

    fn delete<R>(&mut self, byte_range: R) where R: RangeBounds<usize> {
        self.prev = Some(self.content.clone());
        self.content.delete(byte_range);
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
}
    








