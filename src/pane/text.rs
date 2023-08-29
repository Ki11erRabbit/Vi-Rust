use std::io::Write;
use crate::{pane::Pane, window::WindowContentsUtils, cursor::CursorMove};

use std::{collections::HashMap, rc::Rc, cell::RefCell, path::PathBuf, sync::mpsc::Sender, cmp, io};

use crop::{Rope, RopeSlice};
use crossterm::event::KeyEvent;
use crossterm::style::Stylize;

use crate::{cursor::{Cursor, Direction}, mode::{Mode, Normal, Insert, Command}, settings::Settings, window::{Message, WindowContents}, apply_colors};

use super::PaneContainer;


#[derive(Debug, Clone)]
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







pub struct TextPane {
    cursor: Rc<RefCell<Cursor>>,
    file_name: Option<PathBuf>,
    contents: Rope,
    mode: Rc<RefCell<dyn Mode>>,
    modes: HashMap<String, Rc<RefCell<dyn Mode>>>,
    changed: bool,
    settings: Rc<RefCell<Settings>>,
    jump_table: JumpTable,
    sender: Sender<Message>,
}

impl TextPane {
    pub fn new(settings: Rc<RefCell<Settings>>, sender: Sender<Message>) -> Self {
        let mut modes: HashMap<String, Rc<RefCell<dyn Mode>>> = HashMap::new();
        let normal = Rc::new(RefCell::new(Normal::new()));
        normal.borrow_mut().add_keybindings(settings.borrow().mode_keybindings.get("Normal").unwrap().clone());
        normal.borrow_mut().set_key_timeout(settings.borrow().editor_settings.key_timeout);

        let insert = Rc::new(RefCell::new(Insert::new()));
        insert.borrow_mut().add_keybindings(settings.borrow().mode_keybindings.get("Insert").unwrap().clone());
        insert.borrow_mut().set_key_timeout(settings.borrow().editor_settings.key_timeout);
        
        let command = Rc::new(RefCell::new(Command::new()));
        command.borrow_mut().add_keybindings(settings.borrow().mode_keybindings.get("Command").unwrap().clone());
        command.borrow_mut().set_key_timeout(settings.borrow().editor_settings.key_timeout);

        modes.insert("Normal".to_string(), normal.clone());
        modes.insert("Insert".to_string(), insert.clone());
        modes.insert("Command".to_string(), command.clone());

        
        Self {
            cursor: Rc::new(RefCell::new(Cursor::new((0,0)))),
            file_name: None,
            contents: Rope::new(),
            mode: normal,
            modes,
            changed: false,
            settings,
            jump_table: JumpTable::new(),
            sender,
        }
    }


    pub fn set_changed(&mut self, changed: bool) {
        self.changed = changed;
    }


    fn get_row(&self, row: usize, offset: usize, col: usize) -> Option<RopeSlice> {
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

    pub fn borrow_buffer(&self) -> &Rope {
        &self.contents
    }

    pub fn borrow_buffer_mut(&mut self) -> &mut Rope {
        &mut self.contents
    }

    pub fn get_mode(&self, name: &str) -> Option<Rc<RefCell<dyn Mode>>> {
        self.modes.get(name).map(|m| m.clone())
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

}
impl Pane for TextPane {


    fn draw_row(&self, mut index: usize, container: &PaneContainer, output: &mut WindowContents) {
        let rows = container.get_size().1;
        let mut cols = container.get_size().0;

        let ((x1, y1), _) = container.get_corners();

        if self.settings.borrow().editor_settings.border {

            let color_settings = &self.settings.borrow().colors.ui;
            
            if index == 0 && y1 != 0 {
                output.push_str(apply_colors!("-".repeat(cols), color_settings));
                return;
            }
            else {
                index = index;
            }

            if x1 != 0 {
                output.push_str(apply_colors!("|", color_settings));
                cols = cols.saturating_sub(1);
            }
        }

        let real_row = self.cursor.borrow().row_offset + index;
        let col_offset = self.cursor.borrow().col_offset;

        let mut number_of_lines = self.borrow_buffer().line_len();
        if let Some('\n') = self.borrow_buffer().chars().last() {
            number_of_lines += 1;
        }

        let mut num_width = 0;

        if self.settings.borrow().editor_settings.line_number {

            let color_settings = &self.settings.borrow().colors.ui;
            

            if !self.settings.borrow().editor_settings.relative_line_number {


                let mut places = 1;
                while places <= number_of_lines {
                    places *= 10;
                    num_width += 1;
                }

                if real_row + 1 <= number_of_lines {
                    output.push_str(apply_colors!(format!("{:width$}", real_row + 1, width = num_width), color_settings));
                }

            }
            else if self.settings.borrow().editor_settings.relative_line_number {

                let mut places = 1;
                num_width = 3;
                while places <= number_of_lines {
                    places *= 10;
                    num_width += 1;
                }
                if real_row == self.cursor.borrow().get_cursor().1 && real_row + 1 <= number_of_lines {
                    output.push_str(apply_colors!(format!("{:<width$}", real_row + 1 , width = num_width), color_settings));
                }
                else if real_row + 1 <= number_of_lines {
                    output.push_str(apply_colors!(format!("{:width$}",
                                            ((real_row) as isize - (self.cursor.borrow().get_cursor().1 as isize)).abs() as usize,
                                            width = num_width), color_settings));
                }
            }

        }

        self.cursor.borrow_mut().number_line_size = num_width;

        let color_settings = &self.settings.borrow().colors.pane;


        

        if let Some(row) = self.get_row(real_row, col_offset, cols) {
            let mut count = 0;
            row.chars().for_each(|c| if count != (cols - num_width) {
                match c {
                    '\t' => {
                        count += self.settings.borrow().editor_settings.tab_size;
                        output.push_str(apply_colors!(" ".repeat(self.settings.borrow().editor_settings.tab_size), color_settings));
                    },
                    '\n' => output.push_str(apply_colors!(" ", color_settings)),
                    c => {
                        count += 1;
                        output.push_str(apply_colors!(c.to_string(), color_settings));
                    },
                }
            }
                                 else {
                                     output.push_str("");
            });

            output.push_str(apply_colors!(" ".repeat(cols.saturating_sub(count + num_width)), color_settings));
        }
        else if real_row >= number_of_lines {
            output.push_str(apply_colors!(" ".repeat(cols), color_settings));
        }
        else {
            output.push_str(apply_colors!(" ".repeat(cols.saturating_sub(num_width)), color_settings));
        }
        /*output.push_str(" "
                            .attribute(color_settings.attributes)
                            .with(color_settings.foreground_color)
                            .on(color_settings.background_color)
                            .underline(color_settings.underline_color)
        );*/
    }

    fn scroll_cursor(&mut self, container: &PaneContainer) {
        let cursor = self.cursor.clone();

        cursor.borrow_mut().scroll(container);
        
    }


    fn refresh(&mut self) {
        self.mode.borrow_mut().refresh();
    }

    fn save_buffer(&mut self) -> io::Result<()> {
        if let Some(file_name) = &self.file_name {
            let mut file = std::fs::File::create(file_name)?;
            file.write_all(self.contents.to_string().as_bytes())?;
        }
        Ok(())
    }

    fn open_file(&mut self, filename: &PathBuf) -> io::Result<()> {
        let file = std::fs::read_to_string(filename)?;
        self.contents = Rope::from(file);
        self.file_name = Some(PathBuf::from(filename));
        Ok(())
    }

    fn process_keypress(&mut self, key: KeyEvent) -> io::Result<bool> {
        let mode = self.mode.clone();
        let result = mode.borrow_mut().process_keypress(key, self);
        result
    }


    fn get_status(&self, container: &PaneContainer) -> (String, String, String) {
        self.mode.borrow_mut().update_status(container)
    }

    fn change_mode(&mut self, name: &str) {
        if let Some(mode) = self.get_mode(name) {
            self.mode = mode;
        }
    }


    fn run_command(&mut self, command: &str) {
        let mut command_args = command.split_whitespace();
        let command = command_args.next().unwrap_or("");
        match command {
            "q" => {
                if self.changed {
                } else {
                    self.sender.send(Message::ClosePane).unwrap();
                }
            },
            "w" => {
                if let Some(file_name) = command_args.next() {
                    self.file_name = Some(PathBuf::from(file_name));
                }

                self.save_buffer().expect("Failed to save file");
            },
            "w!" => {
                if let Some(file_name) = command_args.next() {
                    self.file_name = Some(PathBuf::from(file_name));
                }

                self.save_buffer().expect("Failed to save file");
            },
            "wq" => {
                self.save_buffer().expect("Failed to save file");
                self.sender.send(Message::ClosePane).unwrap();
            },
            "q!" => {
                self.sender.send(Message::ClosePane).unwrap();
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

                self.cursor.borrow_mut().move_cursor(direction, amount, self);
            },
            "mode" => {
                let mode = command_args.next().unwrap_or("Normal");
                self.change_mode(mode);
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
            "qa!" => {
                self.sender.send(Message::ForceQuitAll).expect("Failed to send message");
            },
            "pane_up" => {
                self.sender.send(Message::PaneUp).expect("Failed to send message");
            },
            "pane_down" => {
                self.sender.send(Message::PaneDown).expect("Failed to send message");
            },
            "pane_left" => {
                self.sender.send(Message::PaneLeft).expect("Failed to send message");
            },
            "pane_right" => {
                self.sender.send(Message::PaneRight).expect("Failed to send message");
            },
            "e" => {
                if let Some(file_name) = command_args.next() {
                    self.sender.send(Message::OpenFile(file_name.to_string())).expect("Failed to send message");
                }
            },

            _ => {}
        }

    }

    fn insert_newline(&mut self) {
        self.insert_char('\n');
        let mut cursor = self.cursor.borrow_mut();

        cursor.move_cursor(Direction::Down, 1, self);
        cursor.set_cursor(CursorMove::ToStart, CursorMove::Nothing, self, (0,0));
    }

    ///TODO: add check to make sure we have a valid byte range
    fn delete_char(&mut self) {
        self.set_changed(true);
        let byte_pos = self.get_byte_offset();

        if byte_pos >= self.contents.byte_len() {
            return;
        }

        self.contents.delete(byte_pos..byte_pos.saturating_add(1));
    }

    ///TODO: add check to make sure we have a valid byte range
    fn backspace_char(&mut self) {
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
            cursor.move_cursor(Direction::Up, 1, self);
            cursor.set_cursor(CursorMove::ToEnd, CursorMove::Nothing, self, (0, 1));
        }
        else {
            cursor.move_cursor(Direction::Left, 1, self);
        }
        

        self.contents.delete(byte_pos.saturating_sub(1)..byte_pos);
    }

    fn insert_char(&mut self, c: char) {
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

    fn insert_str(&mut self, s: &str) {
        self.set_changed(true);
        let byte_pos = self.get_byte_offset();
        if self.contents.chars().count() == 0 {
            self.contents.insert(0, s);
            return;
        }
        let byte_pos = if byte_pos >= self.contents.byte_len() {
            self.contents.byte_len()
        } else {
            byte_pos
        };
        self.contents.insert(byte_pos, s);
    }

    fn get_cursor(&self) -> Rc<RefCell<Cursor>> {
        self.cursor.clone()
    }

    fn get_line_count(&self) -> usize {
        let mut number_of_lines = self.contents.line_len();

        if let Some('\n') = self.contents.chars().last() {
            number_of_lines += 1;
        }
        number_of_lines
    }

    fn buffer_to_string(&self) -> String {
        self.contents.to_string()
    }

    fn get_row_len(&self, row: usize) -> Option<usize> {
        if let Some(line) = self.contents.lines().nth(row) {
            Some(line.chars().count())
        }
        else {
            None
        }
    }


    fn get_filename(&self) -> &Option<PathBuf> {
        &self.file_name
    }

    fn resize_cursor(&mut self, size: (usize, usize)) {
        let mut cursor = self.cursor.borrow_mut();
        cursor.resize(size);
    }

    fn set_cursor_size(&mut self, size: (usize, usize)) {
        let mut cursor = self.cursor.borrow_mut();
        cursor.set_size(size);
    }

}
