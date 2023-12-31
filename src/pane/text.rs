use crate::editor::RegisterType;
use crate::mode::PromptType;
use crate::registers::Registers;
use crate::window::TextRow;
use crate::{pane::Pane, window::StyledChar, cursor::CursorMove, buffer::Buffer};
use std::{io::Write, sync::mpsc::Receiver};

use std::{collections::HashMap, rc::Rc, cell::RefCell, path::PathBuf, sync::mpsc::Sender, io};

use crop::{RopeSlice, Rope};
use crossterm::event::KeyEvent;

use crate::{cursor::{Cursor, Direction}, mode::{Mode, base::{Normal, Insert, Command}}, settings::Settings, window::Message};

use super::{PaneContainer, PaneMessage, popup::PopUpPane};


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

    pub fn add(&mut self, mut cursor: Cursor) {
        if self.index < self.table.len() {
            self.table.truncate(self.index);
        }
        cursor.ignore_offset = false;
        self.table.push(cursor);
        self.index += 1;
    }

    pub fn add_named(&mut self, name: &str, mut cursor: Cursor) {
        cursor.ignore_offset = false;
        self.named.insert(name.to_owned(), cursor);
    }

    pub fn next_jump(&mut self) -> Option<Cursor> {
        if self.index < self.table.len() - 1 {
            self.index += 1;
            if self.table.len() > self.index {
                Some(self.table[self.index])
            }
            else {
                None
            }
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
            let mut j_cursor = self.table[self.index];
            j_cursor.prepare_jump(&cursor);

            self.index = index;
            self.table.truncate(self.index + 1);
            self.table.push(cursor);
            Some(j_cursor)
        }
        else {
            None
        }
    }

    pub fn named_jump(&mut self, name: &str, mut cursor: Cursor) -> Option<Cursor> {
        cursor.ignore_offset = false;
        if let Some(index) = self.named.get(name).cloned() {
            let mut j_cursor = index;
            j_cursor.prepare_jump(&cursor);

            self.add(cursor);
            Some(j_cursor)
        }
        else {
            None
        }
    }
}



pub enum Waiting {
    JumpTarget,
    JumpPosition,
    Completion,
    Goto,
    None,
}



pub struct PlainTextPane {
    cursor: Rc<RefCell<Cursor>>,
    file_name: Option<PathBuf>,
    contents: Buffer,
    mode: Rc<RefCell<dyn Mode>>,
    modes: HashMap<String, Rc<RefCell<dyn Mode>>>,
    changed: bool,
    settings: Rc<RefCell<Settings>>,
    jump_table: JumpTable,
    sender: Sender<Message>,
    popup_channels: Option<(Sender<PaneMessage>, Receiver<PaneMessage>)>,
    waiting: Waiting,
}

impl PlainTextPane {
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
            contents: Buffer::new(settings.clone()),
            mode: normal,
            modes,
            changed: false,
            settings,
            jump_table: JumpTable::new(),
            sender,
            popup_channels: None,
            waiting: Waiting::None,
        }
    }


    pub fn set_changed(&mut self, changed: bool) {
        self.changed = changed;
    }


    fn get_row(&self, row: usize, offset: usize, col: usize) -> Option<RopeSlice> {

        self.contents.get_row(row, offset, col)
    }

    pub fn borrow_buffer(&self) -> &Buffer {
        &self.contents
    }

    pub fn borrow_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.contents
    }

    pub fn get_mode(&self, name: &str) -> Option<Rc<RefCell<dyn Mode>>> {
        self.modes.get(name).map(|m| m.clone())
    }


    fn get_byte_offset(&self) -> Option<usize> {
        let (x, y) = self.cursor.borrow().get_cursor();

        self.contents.get_byte_offset(x, y)
    }

    fn check_messages(&mut self, container: &PaneContainer) {
        match self.popup_channels.as_ref() {
            None => {},
            Some((_, receiver)) => {
                match receiver.try_recv() {
                    Ok(message) => {
                        match message {
                            PaneMessage::String(string) => {

                                match self.waiting {
                                    Waiting::JumpTarget => {
                                        self.waiting = Waiting::JumpPosition;
                                        let command = format!("jump {}", string);
                                        self.run_command(&command, container);
                                    },
                                    Waiting::JumpPosition => {
                                        self.waiting = Waiting::None;
                                        let command = format!("set_jump {}", string);
                                        self.run_command(&command, container);
                                    },
                                    Waiting::Completion => {},
                                    Waiting::Goto => {},
                                    Waiting::None => {
                                    },
                                }
                            },
                            PaneMessage::Close => self.run_command("q!", container),
                        }
                    },
                    Err(_) => {},
                }
            }
        }
    }

}
impl Pane for PlainTextPane {

    fn execute_command(&mut self, command: &str, container: &mut PaneContainer) {
        let mode = self.mode.clone();
        mode.borrow_mut().execute_command(command, self, container);
    }
    
    fn changed(&mut self) {
        self.cursor.borrow_mut().set_moved();
    }

    fn reset(&mut self) {
        self.cursor.borrow_mut().reset_move();
    }

    fn draw_row(&self, mut index: usize, container: &PaneContainer, output: &mut TextRow) {
        //let rows = container.get_size().1;
        let cols = container.get_size().0;


        let ((x1, y1), _) = container.get_corners();

        if self.settings.borrow().editor_settings.border {

            let color_settings = &self.settings.borrow().colors.ui;
            
            if index == 0 && y1 != 0 {

                if !self.cursor.borrow().get_moved() {
                    //eprintln!("Not Changed");
                    for _ in 0..cols {
                        output.push(None);
                    }
                    return;
                }

                
                let string = "-".repeat(cols);

                for c in string.chars() {
                    output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                }
                
                //output.push_str(apply_colors!("-".repeat(cols), color_settings));
                return;
            }
            else {
                let ((_, y), _) = container.get_corners();

                if y != 0 {
                    index = index.saturating_sub(1);
                }
            }

            if x1 != 0 {
                let string = "|".to_string();

                for c in string.chars() {
                    output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                }
                
                //cols = cols.saturating_sub(1);
            }
        }

        let real_row = self.cursor.borrow().row_offset + index;
        let col_offset = self.cursor.borrow().col_offset;

        let number_of_lines = self.contents.get_line_count();

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
                    if !self.cursor.borrow().get_scrolled() {
                        for _ in 0..num_width {
                            output.push(None);
                        }
                    }
                    else {
                        

                        let string = format!("{:width$}", real_row + 1, width = num_width);

                        for c in string.chars() {
                            output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                        }
                    }
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
                    let string = format!("{:<width$}", real_row + 1 , width = num_width);

                    for c in string.chars() {
                        output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                    }
                }
                else if real_row + 1 <= number_of_lines {
                    let string = format!("{:width$}",
                                            ((real_row) as isize - (self.cursor.borrow().get_cursor().1 as isize)).abs() as usize,
                                            width = num_width);

                    for c in string.chars() {
                        output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                    }
                }
            }

        }

        self.cursor.borrow_mut().number_line_size = num_width;

        let color_settings = &self.settings.borrow().colors.pane;


        
        if !self.cursor.borrow().get_scrolled() {
            //eprintln!("Not Changed");
            for _ in 0..(cols - num_width) {
                output.push(None);
            }
            return;
        }

        if let Some(row) = self.get_row(real_row, col_offset, cols) {
            let mut count = 0;
            row.chars().for_each(|c| if count != (cols - num_width) {
                match c {
                    '\t' => {

                        let string = " ".repeat(self.settings.borrow().editor_settings.tab_size);
                        
                        count += self.settings.borrow().editor_settings.tab_size;

                        for c in string.chars() {
                            output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                        }
                    },
                    '\n' => {
                        let string = " ".to_string();

                        for c in string.chars() {
                            output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                        }
                    },
                    c => {
                        count += 1;
                        let string = c.to_string();

                        for c in string.chars() {
                            output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                        }
                    },
                }
            }
                                 else {
            });

            let string = " ".repeat(cols.saturating_sub(count + num_width));

            for c in string.chars() {
                output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
            }
        }
        else if real_row >= number_of_lines {
            let string = " ".repeat(cols);

            for c in string.chars() {
                output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
            }
        }
        else {
            let string = " ".repeat(cols.saturating_sub(num_width));

            for c in string.chars() {
                output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
            }
        }
    }

    fn refresh(&mut self, container: &mut PaneContainer) {
        self.mode.borrow_mut().refresh();
        self.check_messages(container);
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
        self.contents = Buffer::from(file);
        self.file_name = Some(PathBuf::from(filename));
        Ok(())
    }

    fn process_keypress(&mut self, key: KeyEvent, container: &mut PaneContainer) -> io::Result<bool> {
        let mode = self.mode.clone();
        let result = mode.borrow_mut().process_keypress(key, self, container);
        result
    }

    fn scroll_cursor(&mut self, container: &PaneContainer) {
        let cursor = self.cursor.clone();

        cursor.borrow_mut().scroll(container);
        
    }


    fn get_status(&self, container: &PaneContainer) -> (String, String, String) {
        self.mode.borrow_mut().update_status(self, container)
    }

    fn run_command(&mut self, command: &str, container: &PaneContainer) {
        let mut command_args = command.split_whitespace();
        let command = command_args.next().unwrap_or("");
        match command {
            "q" => {
                if self.changed {
                } else {
                    self.sender.send(Message::ClosePane(false, None)).unwrap();
                }
            },
            "w" => {
                if let Some(file_name) = command_args.next() {
                    self.file_name = Some(PathBuf::from(file_name));
                }

                self.save_buffer().expect("Failed to save file");
                self.contents.add_new_rope();
            },
            "w!" => {
                if let Some(file_name) = command_args.next() {
                    self.file_name = Some(PathBuf::from(file_name));
                }

                self.save_buffer().expect("Failed to save file");
                self.contents.add_new_rope();
            },
            "wq" => {
                self.save_buffer().expect("Failed to save file");
                self.sender.send(Message::ClosePane(false, None)).unwrap();
            },
            "q!" => {
                self.sender.send(Message::ClosePane(false, None)).unwrap();
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
                self.contents.add_new_rope();
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
                //eprintln!("Setting jump");
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
                self.contents.add_new_rope();
            },
            "vertical_split" => {
                self.sender.send(Message::VerticalSplit).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "qa!" => {
                self.sender.send(Message::ForceQuitAll).expect("Failed to send message");
            },
            "pane_up" => {
                self.sender.send(Message::PaneUp).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "pane_down" => {
                self.sender.send(Message::PaneDown).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "pane_left" => {
                self.sender.send(Message::PaneLeft).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "pane_right" => {
                self.sender.send(Message::PaneRight).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "e" => {
                if let Some(file_name) = command_args.next() {
                    self.sender.send(Message::OpenFile(file_name.to_string(), None)).expect("Failed to send message");
                }
                self.contents.add_new_rope();
            },
            "prompt_jump" => {
                let (send, recv) = std::sync::mpsc::channel();
                let (send2, recv2) = std::sync::mpsc::channel();

                self.popup_channels = Some((send2, recv));

                let txt_prompt = PromptType::Text(String::new(), None, false);
                let prompt = vec!["Enter Jump".to_string(), "Target".to_string()];

                let pane = PopUpPane::new_prompt(
                    self.settings.clone(),
                    prompt,
                    self.sender.clone(),
                    send,
                    recv2,
                    vec![txt_prompt],
                    true
                );

                let pane = Rc::new(RefCell::new(pane));

                let (_, (x2, y2)) = container.get_corners();
                let (x, y) = container.get_size();

                let (x, y) = (x / 2, y / 2);

                let pos = (x2 - 14 - x, y2 - 6 - y);


                let max_size = container.get_size();
                
                let mut container = PaneContainer::new(max_size, (14, 5), pane, self.settings.clone());


                container.set_position(pos);
                container.set_size((14, 5));



                self.sender.send(Message::CreatePopup(container, true)).expect("Failed to send message");
                self.waiting = Waiting::JumpTarget;

                self.contents.add_new_rope();
            },
            "prompt_set_jump" => {
                let (send, recv) = std::sync::mpsc::channel();
                let (send2, recv2) = std::sync::mpsc::channel();

                self.popup_channels = Some((send2, recv));

                let txt_prompt = PromptType::Text(String::new(), None, false);
                let prompt = vec!["Name the".to_string(), "Target".to_string()];

                let pane = PopUpPane::new_prompt(
                    self.settings.clone(),
                    prompt,
                    self.sender.clone(),
                    send,
                    recv2,
                    vec![txt_prompt],
                    true
                );

                let pane = Rc::new(RefCell::new(pane));

                let (_, (x2, y2)) = container.get_corners();
                let (x, y) = container.get_size();

                let (x, y) = (x / 2, y / 2);

                let pos = (x2 - 14 - x, y2 - 6 - y);


                let max_size = container.get_size();
                
                let mut container = PaneContainer::new(max_size, (14, 5), pane, self.settings.clone());


                container.set_position(pos);
                container.set_size((14, 5));



                self.sender.send(Message::CreatePopup(container, true)).expect("Failed to send message");
                self.waiting = Waiting::JumpPosition;

                self.contents.add_new_rope();
            },
            "undo" => {
                self.contents.undo();

                self.cursor.borrow_mut().number_line_size = self.contents.get_line_count();

                self.cursor.borrow_mut().set_cursor(CursorMove::Nothing, CursorMove::Amount(self.contents.get_line_count()), self, (0,0));
                
            },
            "redo" => {
                self.contents.redo();
                self.cursor.borrow_mut().number_line_size = self.contents.get_line_count();

            },
            "change_tab" => {
                if let Some(tab) = command_args.next() {
                    if let Ok(tab) = tab.parse::<usize>() {
                        self.sender.send(Message::NthTab(tab)).expect("Failed to send message");
                    }
                    else {
                        match tab {
                            "prev" => {
                                self.sender.send(Message::PreviousTab).expect("Failed to send message");
                            },
                            "next" => {
                                self.sender.send(Message::NextTab).expect("Failed to send message");
                            },
                            _ => {}
                        }
                    }
                }
            },
            "open_tab" => {
                self.sender.send(Message::OpenNewTab).expect("Failed to send message");
            },
            "open_tab_with_pane" => {
                self.sender.send(Message::OpenNewTabWithPane).expect("Failed to send message");
            },
            "paste" => {
                
                if let Some(arg) = command_args.next() {
                    if let Ok(number) = arg.parse::<usize>() {
                        let message = Message::Paste(RegisterType::Number(number));

                        self.sender.send(message).expect("Failed to send message");
                    } else {
                        let message = Message::Paste(RegisterType::Name(arg.to_string()));

                        self.sender.send(message).expect("Failed to send message");
                    }
                } else {
                    let message = Message::Paste(RegisterType::None);

                    self.sender.send(message).expect("Failed to send message");
                }
                

            },
            "copy" => {
                eprintln!("Copy");
                if let Some(way) = command_args.next() {

                    let reg = if let Some(arg) = command_args.next() {
                        let reg = if let Ok(number) = arg.parse::<usize>() {
                            RegisterType::Number(number)
                        } else {
                            RegisterType::Name(arg.to_string())
                        };
                        reg
                    } else {
                        RegisterType::None
                    };

                    eprintln!("Register: {:?}", reg);

                    match way {
                        "line" => {
                            let (_, y) = self.cursor.borrow().get_cursor();
                            let (width, _) = container.get_size();
                            
                            let row = self.contents.get_row(y, 0, width);

                            let row = match row {
                                Some(row) => row.to_string(),
                                None => String::new(),
                            };

                            let message = Message::Copy(reg, row);

                            self.sender.send(message).expect("Failed to send message");
                                    
                        },
                        _ => {},

                    }
                        

                }

            },

            _ => {}
        }

    }


    fn change_mode(&mut self, name: &str) {
        if let Some(mode) = self.get_mode(name) {
            self.mode = mode;
        }
    }

    fn insert_newline(&mut self) {
        self.insert_char('\n');
        let mut cursor = self.cursor.borrow_mut();

        cursor.move_cursor(Direction::Down, 1, self);
        cursor.set_cursor(CursorMove::ToStart, CursorMove::Nothing, self, (0,0));
    }

    fn insert_char(&mut self, c: char) {
        self.set_changed(true);
        let byte_pos = self.get_byte_offset();
        let c = c.to_string();
        if self.contents.get_char_count() == 0 {
            self.contents.insert(0, c);
            return;
        }
        let byte_pos = match byte_pos {
            None => self.contents.get_byte_count(),
            Some(byte_pos) => byte_pos,
        };
        
        self.contents.insert_current(byte_pos, c);
    }

    fn insert_str(&mut self, s: &str) {
        self.set_changed(true);
        let byte_pos = self.get_byte_offset();
        if self.contents.get_char_count() == 0 {
            self.contents.insert(0, s);
            return;
        }
        let byte_pos = match byte_pos {
            None => self.contents.get_byte_count(),
            Some(byte_pos) => byte_pos,
        };
        self.contents.insert(byte_pos, s);
    }

    ///TODO: add check to make sure we have a valid byte range
    fn delete_char(&mut self) {
        self.set_changed(true);
        let byte_pos = self.get_byte_offset();

        let byte_pos = match byte_pos {
            None => return,
            Some(byte_pos) => byte_pos,
        };

        self.contents.delete(byte_pos..byte_pos.saturating_add(1));
    }

    ///TODO: add check to make sure we have a valid byte range
    fn backspace_char(&mut self) {
        self.set_changed(true);
        let byte_pos = self.get_byte_offset();
        let mut go_up = false;

        let byte_pos = match byte_pos {
            None => return,
            Some(byte_pos) => byte_pos,
        };

        if self.borrow_buffer().get_nth_byte(byte_pos.saturating_sub(1)) == Some(b'\n') {
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

    fn get_cursor(&self) -> Rc<RefCell<Cursor>> {
        self.cursor.clone()
    }

    fn get_line_count(&self) -> usize {

        self.contents.get_line_count()
    }

    fn buffer_to_string(&self) -> String {
        self.contents.to_string()
    }

    fn get_row_len(&self, row: usize) -> Option<usize> {
        self.contents.line_len(row)
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

    fn backup_buffer(&mut self) {
        self.contents.add_new_rope();
    }


    fn get_settings(&self) -> Rc<RefCell<Settings>> {
        self.settings.clone()
    }


    fn borrow_buffer(&self) -> &Buffer {
        &self.contents
    }
    fn borrow_mut_buffer(&mut self) -> &mut Buffer {
        &mut self.contents
    }


    fn set_sender(&mut self, sender: Sender<Message>) {
        self.sender = sender;
    }
}
