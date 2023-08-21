use std::cmp;
use std::{io, ops::Range};
use std::io::Write;

use crop::{Rope, RopeSlice};
use crossterm::{terminal::{self, ClearType}, execute, cursor, queue};




const TAB_SIZE: usize = 4;


pub struct Window {
    size: (usize, usize),
    contents: WindowContents,
    pane: Pane,
}

impl Window {
    pub fn new() -> Self {
        let win_size = terminal::size()
            .map(|(w, h)| (w as usize, h as usize - 1))// -1 for trailing newline
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
            let real_row = i;//TODO add offset

            let offset = 0;//TODO add offset
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

    pub fn refresh_screen(&mut self) -> io::Result<()> {
        //TODO: add cursor movement

        self.draw_rows();

        //TODO: change cursor position

        self.contents.flush()
    }

    pub fn open_file(&mut self, filename: &str) -> io::Result<()> {
        self.pane.open_file(filename)
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
}


impl Pane {
    pub fn new(size: (usize, usize)) -> Self {
        Self {
            size,
            contents: Rope::new(),
        }
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
}
