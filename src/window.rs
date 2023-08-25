use std::cell::RefCell;
use std::cmp;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::io;
use std::io::Write;
use std::sync::mpsc::{Sender, Receiver, self};
use std::time::Duration;

use crop::{Rope, RopeSlice};
use crossterm::event::{KeyEvent, self, Event};
use crossterm::{terminal::{self, ClearType}, execute, cursor, queue};

use crate::mode::{Mode, Normal, Insert, Command};
use crate::cursor::{Cursor, Direction, CursorMove};
use crate::settings::{Settings, Keys};



pub enum Message {
    HorizontalSplit,
    VerticalSplit,
}

pub struct Window {
    size: (usize, usize),
    contents: WindowContents,
    active_pane: usize,
    panes: Vec<Rc<RefCell<Pane>>>,
    settings: Settings,
    duration: Duration,
    channels: (Sender<Message>, Receiver<Message>),
}

impl Window {
    pub fn new() -> Self {
        let settings = Settings::default();
        
        let duration = Duration::from_millis(settings.editor_settings.key_timeout);

        let channels = mpsc::channel();
        
        let win_size = terminal::size()
            .map(|(w, h)| (w as usize, h as usize - 2))// -1 for trailing newline and -1 for command bar
            .unwrap();
        let pane = Rc::new(RefCell::new(Pane::new(win_size, settings.clone(), channels.0.clone())));

        let panes = vec![pane.clone()];
        Self {
            size: win_size,
            contents: WindowContents::new(),
            active_pane: 0,
            panes,
            duration,
            settings,
            channels,
        }
    }

    fn remove_panes(&mut self) {
        let mut panes_to_remove = Vec::new();
        for (i, pane) in self.panes.iter().enumerate() {
            if pane.borrow().close {
                panes_to_remove.push(i);
            }
        }

        
        for i in panes_to_remove.iter().rev() {
            let pane_size = self.panes[*i].borrow().size;
            let ((rstart_x, rstart_y), (rend_x, rend_y)) = self.panes[*i].borrow().get_corners();
            
            loop {
                if *i + 1 < self.panes.len() {
                    let corners = self.panes[*i].borrow().get_corners();
                    if self.panes[*i + 1].borrow_mut().combine(corners) {
                        break;
                    }
                }
                if *i != 0 {
                    let corners = self.panes[*i].borrow().get_corners();
                    if self.panes[*i - 1].borrow_mut().combine(corners) {
                        break;
                    }
                }
                break;
            }

            
            self.panes.remove(*i);
        }
        if self.panes.len() == 0 {
            self.active_pane = 0;
        }
        else {
            self.active_pane = cmp::min(self.active_pane, self.panes.len() - 1);
        }

    }



    fn horizontal_split(&mut self) {
        eprintln!("split panes: {:?}", self.panes.len());
        let active_pane_size = self.panes[self.active_pane].borrow().size;
        let new_pane_size = (active_pane_size.0, active_pane_size.1 / 2);
        self.panes[self.active_pane].borrow_mut().size = new_pane_size;


        let new_pane_index = self.active_pane + 1;
        self.panes.insert(new_pane_index, Rc::new(RefCell::new(Pane::new(new_pane_size, self.settings.clone(), self.channels.0.clone()))));

        let ((x,_), (_, y)) = self.panes[self.active_pane].borrow().get_corners();
        let new_pane_position = (x, y + 1);

        let new_pane = self.panes[new_pane_index].clone();
        new_pane.borrow_mut().set_position(new_pane_position);


        // This is for testing purposes, we need to make sure that we can actually access the new pane
        self.active_pane = new_pane_index;

        eprintln!("split panes: {:?}", self.panes.len());
    }

    fn vertical_split(&mut self) {
        let active_pane_size = self.panes[self.active_pane].borrow().size;
        let new_pane_size = (active_pane_size.0 / 2, active_pane_size.1);
        self.panes[self.active_pane].borrow_mut().size = new_pane_size;


        let new_pane_index = self.active_pane + 1;
        self.panes.insert(new_pane_index, Rc::new(RefCell::new(Pane::new(new_pane_size, self.settings.clone(), self.channels.0.clone()))));

        let ((_,y), (x, _)) = self.panes[self.active_pane].borrow().get_corners();
        let new_pane_position = (x + 1, y);

        let new_pane = self.panes[new_pane_index].clone();
        new_pane.borrow_mut().set_position(new_pane_position);
        

        self.active_pane = new_pane_index;
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

    fn read_messages(&mut self) {
        match self.channels.1.try_recv() {
            Ok(message) => {
                match message {
                    Message::HorizontalSplit => {
                        self.horizontal_split();
                    }
                    Message::VerticalSplit => {
                        self.vertical_split();
                    }
                }
            },
            Err(_) => {}
        }
    }


    pub fn run(&mut self) -> io::Result<bool> {
        self.refresh_screen()?;
        self.read_messages();
        self.remove_panes();
        if self.panes.len() == 0 {
            return Ok(false);
        }
        self.refresh_screen()?;
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


        //eprintln!("panes: {}", self.panes.len());
        let panes = self.panes.len();
        for i in 0..rows {

            let mut pane_index = 0;
            let mut window_index = 0;
            //eprintln!("size: {:?} i: {}", self.size, i);
            while window_index < self.size.0 {
                //eprintln!("window_index: {} pane_index: {}\r\n", window_index, pane_index);
                if pane_index >= self.panes.len() {
                    break;
                }
                //eprintln!("pane size: {:?} pane_index: {}", self.panes[pane_index].borrow().size, pane_index);
                //eprintln!("pane corners: {:?}", self.panes[pane_index].borrow().get_corners());
                let ((start_x, start_y), (end_x, end_y)) = self.panes[pane_index].borrow().get_corners();
                if start_y <= i && end_y >= i {
                    self.contents.merge(&mut self.panes[pane_index].borrow().draw_row(i - start_y));
                    window_index += end_x - start_x + 1;
                }
                pane_index += 1;
            }


            //self.contents.merge(&mut self.panes[self.active_pane].borrow().draw_row(i));

            
            queue!(
                self.contents,
                terminal::Clear(ClearType::UntilNewLine),
            ).unwrap();


            self.contents.push_str("\r\n");

        }

    }


    pub fn draw_status_bar(&mut self) {
        Self::clear_screen().unwrap();
        queue!(
            self.contents,
            terminal::Clear(ClearType::UntilNewLine),
        ).unwrap();
        self.contents.push_str("\r\n");

        let (first, second) = self.panes[self.active_pane].borrow().get_status();
        let total = first.len() + second.len();
        
        self.contents.push_str(first.as_str());
        let remaining = self.size.0.saturating_sub(total);
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
        let x = x + self.panes[self.active_pane].borrow().get_position().0;
        let y = y + self.panes[self.active_pane].borrow().get_position().1;

        
        let x = {
            if let Some(row) = self.panes[self.active_pane].borrow().borrow_buffer().lines().nth(y) {
                let len = row.chars().count();
                //cmp::min(x, len)
                x
            }
            else {
                x
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
        self.index += 1;
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
    position: (usize, usize),
    file_name: Option<PathBuf>,
    contents: Rope,
    mode: Rc<RefCell<dyn Mode>>,
    modes: HashMap<String, Rc<RefCell<dyn Mode>>>,
    pub cursor: Rc<RefCell<Cursor>>,
    close: bool,
    changed: bool,
    settings: Settings,
    jump_table: JumpTable,
    sender: Sender<Message>,
}


impl Pane {
    pub fn new(size: (usize, usize), settings: Settings, sender: Sender<Message>) -> Self {
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
            position: (0, 0),
            file_name: None,
            contents: Rope::new(),
            mode: normal,
            modes,
            cursor: Rc::new(RefCell::new(Cursor::new(size))),
            close: false,
            changed: false,
            settings,
            jump_table: JumpTable::new(),
            sender,
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
            let mut count = 0;
            row.chars().for_each(|c| if count != (cols - num_width) {
                match c {
                    '\t' => {
                        count += self.settings.editor_settings.tab_size;
                        output.push_str(" ".repeat(self.settings.editor_settings.tab_size).as_str())
                    },
                    '\n' => output.push_str(" "),
                    c => {
                        count += 1;
                        output.push(c)
                    },
                }
            }
                                 else {
                                     output.push_str("");
            });

            output.push_str(" ".repeat(cols.saturating_sub(count + num_width)).as_str());

            //output.push_str(" ".repeat(cols - row.chars().count() / 2).as_str());

            /*queue!(
                output,
                terminal::Clear(ClearType::UntilNewLine),
            ).unwrap();*/
        }
        else if real_row >= number_of_lines {
            output.push_str(" ".repeat(cols).as_str());

            /*queue!(
                output,
                terminal::Clear(ClearType::UntilNewLine),
            ).unwrap();*/
        }
        else {
            output.push_str(" ".repeat(cols - num_width).as_str());
        }

        //output.push_str("\r\n");
        output
    }

    pub fn combine(&mut self, corners: ((usize, usize), (usize, usize))) -> bool {
        eprintln!("Combine: {:?}", corners);
        let ((other_start_x, other_start_y), (other_end_x, other_end_y)) = corners;
        eprintln!("Combine: {:?}", self.get_corners());
        let ((start_x, start_y), (end_x, end_y)) = self.get_corners();


        if other_start_y == start_y || other_end_y == end_y {

            
            //Try combining from the left to right
            if end_x + 1 == other_start_x && start_y == other_start_y && end_y == other_end_y {
                let mut width = other_end_x - start_x;
                let mut height = end_y - start_y;
                eprintln!("Width: {}, Height: {}", width, height);


                self.size.0 = width;
                self.size.1 = height;
                return true;
            }

            //Try combining from the right to left
            else if other_start_x - 1 == end_x && start_y == other_start_y && end_y == other_end_y {
                let width = end_x - other_start_x;
                let height = end_y - start_y;
                eprintln!("Width: {}, Height: {}", width, height);

                self.size.0 = width;
                self.size.1 = height;
                return true;
            }
        }
        else if other_start_x == start_x || other_end_x == end_x {

            //Try combining from the top to bottom
            if end_y + 1 == other_start_y && start_x == other_start_x && end_x == other_end_x {
                let width = end_x - start_x;
                let height = other_end_y - start_y;
                eprintln!("Width: {}, Height: {}", width, height);

                self.size.0 = width;
                self.size.1 = height;
                return true;
            }

            //Try combining from the bottom to top
            else if other_start_y - 1 == end_y && start_x == other_start_x && end_x == other_end_x {
                let width = end_x - start_x;
                let height = end_y - other_start_y;
                eprintln!("Width: {}, Height: {}", width, height);

                self.size.0 = width;
                self.size.1 = height;
                return true;
            }
        }

        return false;
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

    pub fn set_size(&mut self, size: (usize, usize)) {
        self.size = size;
    }

    pub fn increase_size(&mut self, size: (usize, usize)) {
        self.size.0 += size.0;
        self.size.1 += size.1;
    }

    pub fn get_row(&self, row: usize, offset: usize, col: usize) -> Option<RopeSlice> {
        if row >= self.contents.line_len() {
            return None;
        }
        let line = self.contents.line(row);
        let len = cmp::min(col + offset, line.line_len().saturating_sub(offset));
        if len == 0 {
            return None;
        }
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

    pub fn set_position(&mut self, position: (usize, usize)) {
        self.position = position;
    }

    pub fn get_position(&self) -> (usize, usize) {
        self.position
    }

    pub fn get_corners(&self) -> ((usize, usize), (usize, usize)) {
        let x = self.size.0 + self.position.0;
        let y = self.size.1 + self.position.1;
        (self.position, (x, y))
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
        let byte_pos = if byte_pos >= self.contents.byte_len() {
            self.contents.byte_len()
        } else {
            byte_pos
        };
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
                    Some("line_start") => Direction::LineStart,
                    Some("line_end") => Direction::LineEnd,
                    Some("file_top") => Direction::FileTop,
                    Some("file_bottom") => Direction::FileBottom,
                    Some("page_up") => Direction::PageUp,
                    Some("page_down") => Direction::PageDown,
                    _ => panic!("Invalid direction"),
                };

                let amount = command_args.next().unwrap_or("1").parse::<usize>().unwrap_or(1);

                match direction {
                    Direction::FileBottom | Direction::FileTop | Direction::PageUp | Direction::PageDown => {
                        let cursor = self.cursor.borrow();
                        self.jump_table.add(*cursor);
                    },
                    _ => {},
                }

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
            "horizontal_split" => {
                self.sender.send(Message::HorizontalSplit).expect("Failed to send message");
            },
            "vertical_split" => {
                self.sender.send(Message::VerticalSplit).expect("Failed to send message");
            },

            _ => {}
        }

    }
}
