use std::cell::RefCell;
use std::cmp;
use std::collections::HashMap;
use std::num::Wrapping;
use std::path::PathBuf;
use std::rc::Rc;
use std::io;
use std::io::Write;
use std::time::Duration;

use crop::{Rope, RopeSlice};
use crossterm::event::{KeyEvent, self, Event};
use crossterm::{terminal::{self, ClearType}, execute, cursor, queue};

use crate::mode::{Mode, Normal, Insert, Command};
use crate::cursor::{Cursor, Direction, CursorMove};
use crate::settings::{Settings, Keys};


struct KeyReader {
    pub duration: Duration,
}

impl KeyReader {
    fn read_key(&self) -> io::Result<KeyEvent> {
        loop {
            if event::poll(self.duration)? {
                if let Event::Key(key) = event::read()? {
                    return Ok(key);
                }
            }
        }
    }
}


const TAB_SIZE: usize = 4;


pub struct Window {
    size: (usize, usize),
    contents: WindowContents,
    active_pane: usize,
    panes: Vec<Rc<RefCell<Pane>>>,
    settings: Settings,
    duration: Duration,
}

impl Window {
    pub fn new() -> Self {
        let settings = Settings::default();
        
        let duration = Duration::from_millis(settings.editor_settings.key_timeout);
        
        let win_size = terminal::size()
            .map(|(w, h)| (w as usize, h as usize - 2))// -1 for trailing newline and -1 for command bar
            .unwrap();
        let pane = Rc::new(RefCell::new(Pane::new(win_size, settings.clone())));
        let panes = vec![pane.clone()];
        Self {
            size: win_size,
            contents: WindowContents::new(),
            active_pane: 0,
            panes,
            duration,
            settings,
        }
    }

    fn remove_panes(&mut self) {
        self.panes.retain(|pane| !pane.borrow().close);
        if self.panes.len() == 0 {
            self.active_pane = 0;
        }
        else {
            self.active_pane = cmp::min(self.active_pane, self.panes.len() - 1);
        }
        
    }

    fn read_key(&mut self) -> io::Result<KeyEvent> {
        loop {
            if event::poll(self.duration)? {
                if let Event::Key(key) = event::read()? {
                    return Ok(key);
                }
            }
            self.refresh_screen()?;
        }
    }


    pub fn run(&mut self) -> io::Result<bool> {
        self.refresh_screen()?;
        self.remove_panes();
        if self.panes.len() == 0 {
            return Ok(false);
        }
        let key = self.read_key()?;
        self.process_keypress(key)
    }

    pub fn clear_screen() -> io::Result<()> {
        execute!(std::io::stdout(), terminal::Clear(terminal::ClearType::All))?;
        execute!(std::io::stdout(), cursor::MoveTo(0, 0))
    }

    fn draw_rows(&mut self) {
        let rows = self.size.1;
        let cols = self.size.0;

        for i in 0..rows {
            /*let real_row = i + self.panes[self.active_pane].borrow().cursor.borrow().row_offset;

            let offset = 0 + self.panes[self.active_pane].borrow().cursor.borrow().col_offset;
            if let Some(row) = self.panes[self.active_pane].borrow().get_row(real_row, offset, cols) {
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
            else {
                self.contents.push_str(" ".repeat(cols).as_str());
        }*/

            self.contents.merge(&mut self.panes[self.active_pane].borrow().draw_row(i));

            


            //self.contents.push_str("\r\n");

        }

    }


    pub fn draw_status_bar(&mut self) {
        queue!(
            self.contents,
            terminal::Clear(ClearType::UntilNewLine),
        ).unwrap();
        self.contents.push_str("\r\n");

        let (first, second) = self.panes[self.active_pane].borrow().get_status();
        let total = first.len() + second.len();
        
        self.contents.push_str(first.as_str());
        let remaining = self.size.0 - total;
        self.contents.push_str(" ".to_owned().repeat(remaining).as_str());
        self.contents.push_str(second.as_str());
    }

    pub fn refresh_screen(&mut self) -> io::Result<()> {

        self.panes[self.active_pane].borrow_mut().refresh();

        self.panes[self.active_pane].borrow_mut().scroll_cursor();

        queue!(
            self.contents,
            cursor::Hide,
            cursor::MoveTo(0, 0),
        )?;

        self.draw_rows();
        self.draw_status_bar();

        let (x, y) = self.panes[self.active_pane].borrow().cursor.borrow().get_real_cursor();

        
        let x = {
            if let Some(row) = self.panes[self.active_pane].borrow().borrow_buffer().lines().nth(y) {
                let len = row.chars().count();
                cmp::min(x, len)
            }
            else {
                0
            }
        } + self.panes[self.active_pane].borrow().cursor.borrow().number_line_size;

        queue!(
            self.contents,
            cursor::MoveTo(x as u16, y as u16),
            cursor::Show,
        )?;

        self.contents.flush()
    }

    pub fn open_file(&mut self, filename: &str) -> io::Result<()> {
        self.panes[self.active_pane].borrow_mut().open_file(filename)
    }

    pub fn process_keypress(&mut self, key: KeyEvent) -> io::Result<bool> {
        self.panes[self.active_pane].borrow_mut().process_keypress(key)
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

    fn merge(&mut self, other: &mut Self) {
        self.content.push_str(other.content.as_str());
        other.content.clear();
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



pub struct JumpTable {
    table: Vec<Cursor>,
    index: usize,
    named: HashMap<String, Cursor>,
}

impl JumpTable {
    pub fn new() -> Self {
        Self {
            table: Vec::new(),
            index: 0,
            named: HashMap::new(),
        }
    }

    pub fn add(&mut self, cursor: Cursor) {
        if self.index < self.table.len() {
            self.table.truncate(self.index);
        }
        self.table.push(cursor);
    }

    pub fn add_named(&mut self, name: &str, cursor: Cursor) {
        self.named.insert(name.to_owned(), cursor);
    }

    pub fn next_jump(&mut self) -> Option<Cursor> {
        if self.index < self.table.len() - 1 {
            self.index += 1;
            Some(self.table[self.index])
        }
        else {
            None
        }
    }

    pub fn prev_jump(&mut self) -> Option<Cursor> {
        if self.index > 0 {
            self.index -= 1;
            Some(self.table[self.index])
        }
        else {
            None
        }
    }

    pub fn jump(&mut self, index: usize, cursor: Cursor) -> Option<Cursor> {
        if index < self.table.len() {
            self.index = index;
            self.table.truncate(self.index + 1);
            self.table.push(cursor);
            Some(self.table[self.index])
        }
        else {
            None
        }
    }

    pub fn named_jump(&mut self, name: &str, cursor: Cursor) -> Option<Cursor> {
        if let Some(index) = self.named.get(name).cloned() {
            self.add(cursor);
            Some(index)
        }
        else {
            None
        }
    }
}


pub struct Pane {
    size: (usize, usize),
    file_name: Option<PathBuf>,
    contents: Rope,
    mode: Rc<RefCell<dyn Mode>>,
    modes: HashMap<String, Rc<RefCell<dyn Mode>>>,
    pub cursor: Rc<RefCell<Cursor>>,
    close: bool,
    changed: bool,
    settings: Settings,
    jump_table: JumpTable,
}


impl Pane {
    pub fn new(size: (usize, usize), settings: Settings) -> Self {
        let mut modes: HashMap<String, Rc<RefCell<dyn Mode>>> = HashMap::new();
        let normal = Rc::new(RefCell::new(Normal::new()));
        normal.borrow_mut().add_keybindings(settings.mode_keybindings.get("Normal").unwrap().clone());
        normal.borrow_mut().set_key_timeout(settings.editor_settings.key_timeout);

        let insert = Rc::new(RefCell::new(Insert::new()));
        insert.borrow_mut().add_keybindings(settings.mode_keybindings.get("Insert").unwrap().clone());
        insert.borrow_mut().set_key_timeout(settings.editor_settings.key_timeout);
        
        let command = Rc::new(RefCell::new(Command::new()));
        command.borrow_mut().add_keybindings(settings.mode_keybindings.get("Command").unwrap().clone());
        command.borrow_mut().set_key_timeout(settings.editor_settings.key_timeout);

        modes.insert("Normal".to_string(), normal.clone());
        modes.insert("Insert".to_string(), insert.clone());
        modes.insert("Command".to_string(), command.clone());
        Self {
            size,
            file_name: None,
            contents: Rope::new(),
            mode: normal,
            modes,
            cursor: Rc::new(RefCell::new(Cursor::new(size))),
            close: false,
            changed: false,
            settings,
            jump_table: JumpTable::new(),
        }
    }


    pub fn draw_row(&self, index: usize) -> WindowContents {
        let rows = self.size.1;
        let cols = self.size.0;

        let real_row = self.cursor.borrow().row_offset + index;
        let col_offset = self.cursor.borrow().col_offset;

        let mut number_of_lines = self.borrow_buffer().line_len();
        if let Some('\n') = self.borrow_buffer().chars().last() {
            number_of_lines += 1;
        }

        //number_of_lines = self.borrow_buffer().chars().filter(|c| *c == '\n').count();



        let mut output = WindowContents::new();

        let mut num_width = 0;
        if self.settings.editor_settings.line_number && !self.settings.editor_settings.relative_line_number {
            let mut places = 1;
            while places < number_of_lines {
                places *= 10;
                num_width += 1;
            }
            if real_row + 1 <= number_of_lines {
                output.push_str(format!("{:width$}", real_row + 1, width = num_width).as_str());
            }
        }
        else if self.settings.editor_settings.line_number && self.settings.editor_settings.relative_line_number {
            let mut places = 1;
            num_width = 3;
            while places < rows {
                places *= 10;
                num_width += 1;
            }
            if real_row == self.cursor.borrow().get_cursor().1 && real_row + 1 <= number_of_lines {
                output.push_str(format!("{}{:width$}", real_row + 1, ' ', width = num_width - real_row.to_string().chars().count()).as_str());
            }
            else if real_row + 1 <= number_of_lines {
                output.push_str(format!("{:width$}", ((real_row + 1) as isize - (self.cursor.borrow().get_cursor().1 as isize)).abs() as usize, width = num_width).as_str());
            }
        }

        self.cursor.borrow_mut().number_line_size = num_width;


        if let Some(row) = self.get_row(real_row, col_offset, cols) {
            row.chars().for_each(|c| match c {
                '\t' => output.push_str(" ".repeat(self.settings.editor_settings.tab_size).as_str()),
                //'\n' => output.push_str(" "),
                _ => output.push(c),
            });

            queue!(
                output,
                terminal::Clear(ClearType::UntilNewLine),
            ).unwrap();
        }
        else if real_row >= number_of_lines {
            output.push_str(" ".repeat(cols).as_str());
        }

        output.push_str("\r\n");
        output
    }

    pub fn refresh(&mut self) {
        self.mode.borrow_mut().refresh();
    }

    pub fn set_changed(&mut self, changed: bool) {
        self.changed = changed;
    }

    pub fn can_close(&self) -> bool {
        self.close
    }

    pub fn close(&mut self) {
        self.close = true;
    }

    pub fn save_buffer(&mut self) {
        if let Some(file_name) = &self.file_name {
            let mut file = std::fs::File::create(file_name).unwrap();
            file.write_all(self.contents.to_string().as_bytes()).unwrap();
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
        let len = cmp::min(col + offset, line.line_len().saturating_sub(offset));
        Some(line.line_slice(offset..len))
    }

    pub fn open_file(&mut self, filename: &str) -> io::Result<()> {
        let file = std::fs::read_to_string(filename)?;
        self.contents = Rope::from(file);
        self.file_name = Some(PathBuf::from(filename));
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

    pub fn get_status(&self) -> (String, String) {
        self.mode.borrow().update_status(self)
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
        let (x, mut y) = self.cursor.borrow().get_cursor();
        while y > self.contents.line_len() {
            y = y.saturating_sub(1);
        }
        let line_pos = self.contents.byte_of_line(y);
        let row_pos = x;

        let byte_pos = line_pos + row_pos;

        byte_pos
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
        let mut cursor = self.cursor.borrow_mut();

        cursor.move_cursor(Direction::Down, 1, self.borrow_buffer());
        cursor.set_cursor(CursorMove::ToStart, CursorMove::Nothing, self.borrow_buffer(), (0,0));
    }

    ///TODO: add check to make sure we have a valid byte range
    pub fn delete_char(&mut self) {
        self.set_changed(true);
        let byte_pos = self.get_byte_offset();

        if byte_pos >= self.contents.byte_len() {
            return;
        }

        self.contents.delete(byte_pos..byte_pos.saturating_add(1));
    }

    ///TODO: add check to make sure we have a valid byte range
    pub fn backspace(&mut self) {
        self.set_changed(true);
        let byte_pos = self.get_byte_offset();
        let mut go_up = false;

        if self.borrow_buffer().bytes().nth(byte_pos.saturating_sub(1)) == Some(b'\n') {
            go_up = true;
        }

        if byte_pos == 0 {
            return;
        }

        let mut cursor = self.cursor.borrow_mut();

        if go_up {
            cursor.move_cursor(Direction::Up, 1, self.borrow_buffer());
            cursor.set_cursor(CursorMove::ToEnd, CursorMove::Nothing, self.borrow_buffer(), (0, 1));
        }
        else {
            cursor.move_cursor(Direction::Left, 1, self.borrow_buffer());
        }
        

        self.contents.delete(byte_pos.saturating_sub(1)..byte_pos);
    }

    pub fn insert_char(&mut self, c: char) {
        self.set_changed(true);
        let byte_pos = self.get_byte_offset();
        let c = c.to_string();
        if self.contents.chars().count() == 0 {
            self.contents.insert(0, c);
            return;
        }
        self.contents.insert(byte_pos, c);
    }

    pub fn run_command(&mut self, command: &str) {
        let mut command_args = command.split_whitespace();
        let command = command_args.next().unwrap_or("");
        match command {
            "q" => {
                if self.changed {
                } else {
                    self.close();
                }
            },
            "w" => {
                if let Some(file_name) = command_args.next() {
                    self.file_name = Some(PathBuf::from(file_name));
                }

                self.save_buffer();
            },
            "w!" => {
                if let Some(file_name) = command_args.next() {
                    self.file_name = Some(PathBuf::from(file_name));
                }

                self.save_buffer();
            },
            "wq" => {
                self.save_buffer();
                self.close();
            },
            "q!" => {
                self.close();
            },
            "move" => {
                let direction = command_args.next();
                let direction = match direction {
                    Some("up") => Direction::Up,
                    Some("down") => Direction::Down,
                    Some("left") => Direction::Left,
                    Some("right") => Direction::Right,
                    _ => panic!("Invalid direction"),
                };

                let amount = command_args.next().unwrap_or("1").parse::<usize>().unwrap_or(1);

                self.cursor.borrow_mut().move_cursor(direction, amount, self.borrow_buffer());
            },
            "mode" => {
                let mode = command_args.next().unwrap_or("Normal");
                self.set_mode(mode);
            },
            "jump" => {
                if let Some(jump) = command_args.next() {
                    match jump {
                        "next" => {
                            let mut cursor = self.cursor.borrow_mut();
                            if let Some(new_cursor) = self.jump_table.next_jump() {
                                *cursor = new_cursor;
                            }
                        },
                        "prev" => {
                            let mut cursor = self.cursor.borrow_mut();
                            if let Some(new_cursor) = self.jump_table.prev_jump() {
                                *cursor = new_cursor;
                            }
                        },
                        other => {
                            let mut cursor = self.cursor.borrow_mut();
                            if let Some(index) = other.parse::<usize>().ok() {
                                if let Some(new_cursor) = self.jump_table.jump(index, *cursor) {
                                    *cursor = new_cursor;
                                }
                            }
                            else {
                                if let Some(new_cursor) = self.jump_table.named_jump(other, *cursor) {
                                    *cursor = new_cursor;
                                }

                            }

                        }

                    }
                }

            },
            "set_jump" => {
                let cursor = self.cursor.borrow();
                if let Some(jump) = command_args.next() {
                    self.jump_table.add_named(jump, *cursor);
                }
                else {
                    self.jump_table.add(*cursor);
                }
                
            },
            _ => {}
        }

    }
}
