use std::{path::PathBuf, sync::{mpsc::{Sender, Receiver, TryRecvError}, Arc}, collections::{HashSet, HashMap}, rc::Rc, cell::RefCell, io, fs::File};

use tree_sitter::{Tree, Parser};

use crate::{buffer::Buffer, lsp::{LspControllerMessage, lsp_utils::{Diagnostics, CompletionList, LocationResponse, Diagnostic}}, cursor::{Cursor, Direction, CursorMove}, mode::{Mode, base::{Normal, Insert, Command}}, settings::Settings, new_editor::{EditorMessage, StyledChar}, new_window::WindowMessage, treesitter::tree_sitter_scheme};

use super::{Pane, TextBuffer, PaneMessage};




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



pub struct TreeSitterInfo {
    pub parser: Parser,
    pub tree: Tree,
    pub language: String,
}

impl TreeSitterInfo {
    pub fn new(parser: Parser, tree: Tree, language: String) -> Self {
        Self {
            parser,
            tree,
            language,
        }
    }
}

pub struct LspInfo {
    pub lsp_client: (Sender<LspControllerMessage>, Arc<Receiver<LspControllerMessage>>),
    pub file_version: usize,
    pub lsp_diagnostics: Diagnostics,
    pub sent_diagnostics: HashSet<Diagnostic>,
    pub lsp_completion: Option<CompletionList>,
    pub lsp_location: Option<LocationResponse>,
}


impl LspInfo {
    pub fn new(lsp_client: (Sender<LspControllerMessage>, Arc<Receiver<LspControllerMessage>>)) -> Self {
        Self {
            lsp_client,
            file_version: 0,
            lsp_diagnostics: Diagnostics::new(),
            sent_diagnostics: HashSet::new(),
            lsp_completion: None,
            lsp_location: None,
        }
    }

}



pub struct TextPane {
    cursor: Rc<RefCell<Cursor>>,
    file_name: Option<PathBuf>,
    contents: Buffer,
    mode: Rc<RefCell<dyn Mode>>,
    modes: HashMap<String, Rc<RefCell<dyn Mode>>>,
    text_changed: bool,

    settings: Rc<RefCell<Settings>>,
    jump_table: JumpTable,
    waiting: Waiting,

    popup_channels: Vec<(Sender<PaneMessage>, Receiver<PaneMessage>)>,
    current_popup: Option<usize>,

    window_sender: Sender<WindowMessage>,
    window_receiver: Rc<Receiver<WindowMessage>>,

    editor_sender: Sender<EditorMessage>,
    editor_receiver: Rc<Receiver<EditorMessage>>,

    lsp_sender: Sender<LspControllerMessage>,
    lsp_listener: Rc<Receiver<LspControllerMessage>>,
    
    

    tree_sitter_info: Option<TreeSitterInfo>,
    lsp_info: Option<LspInfo>,


}


impl TextPane {
    pub fn new(settings: Rc<RefCell<Settings>>,
               window_sender: Sender<WindowMessage>,
               window_receiver: Rc<Receiver<WindowMessage>>,
               editor_sender: Sender<EditorMessage>,
               editor_receiver: Rc<Receiver<EditorMessage>>,
               lsp_sender: Sender<LspControllerMessage>,
               lsp_listener: Rc<Receiver<LspControllerMessage>>) -> Self {


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
            cursor: Rc::new(RefCell::new(Cursor::new((0, 0)))),
            file_name: None,
            contents: Buffer::new(settings.clone()),
            mode: normal,
            modes,
            text_changed: false,
            settings,
            jump_table: JumpTable::new(),
            waiting: Waiting::None,
            popup_channels: Vec::new(),
            current_popup: None,

            window_sender,
            window_receiver,

            editor_sender,
            editor_receiver,

            lsp_sender,
            lsp_listener,

            tree_sitter_info: None,
            lsp_info: None,
        }
        
    }
}



impl Pane for TextPane {
    fn draw_row(&self, index: usize, container: &super::PaneContainer, output: &mut crate::new_editor::LayerRow) {
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

    fn refresh(&mut self, container: &mut super::PaneContainer) {
        let cursor = self.cursor.clone();

        cursor.borrow_mut().scroll(container);
    }

    fn process_keypress(&mut self, key: crossterm::event::KeyEvent, container: &mut super::PaneContainer) -> std::io::Result<()> {
        let mode = self.mode.clone();
        let result = mode.borrow_mut().process_keypress(key, self, container);

        result
    }

    fn get_status(&self, container: &super::PaneContainer) -> (String, String, String) {
        self.mode.borrow_mut().update_status(self, container)
    }

    fn draw_status(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.cursor.borrow_mut().reset_move();
    }

    fn changed(&mut self) {
        self.cursor.borrow_mut().set_moved();
    }

    fn get_cursor(&self) -> Option<(usize, usize)> {
        Some(self.cursor.borrow().get_real_cursor())
    }

    fn get_name(&self) -> &str {
        match self.file_name {
            Some(ref name) => name.file_name().unwrap().to_str().unwrap(),
            None => "",
        }
    }

    fn run_command(&mut self, command: &str, container: &mut super::PaneContainer) {
        let mut command_args = command.split_whitespace();
        let command = command_args.next().unwrap_or("");
        match command {
            "q" => {
                if self.changed {
                } else {
                    self.sender.send(WindowMessage::ClosePane(false, None)).unwrap();
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
                self.sender.send(WindowMessage::ClosePane(false, None)).unwrap();
            },
            "q!" => {
                self.sender.send(WindowMessage::ClosePane(false, None)).unwrap();
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
                self.sender.send(WindowMessage::HorizontalSplit).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "vertical_split" => {
                self.sender.send(WindowMessage::VerticalSplit).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "qa!" => {
                self.sender.send(WindowMessage::ForceQuitAll).expect("Failed to send message");
            },
            "pane_up" => {
                self.sender.send(WindowMessage::PaneUp).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "pane_down" => {
                self.sender.send(WindowMessage::PaneDown).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "pane_left" => {
                self.sender.send(WindowMessage::PaneLeft).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "pane_right" => {
                self.sender.send(WindowMessage::PaneRight).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "e" => {
                if let Some(file_name) = command_args.next() {
                    self.sender.send(WindowMessage::OpenFile(file_name.to_string(), None)).expect("Failed to send message");
                }
                self.contents.add_new_rope();
            },
            /*"prompt_jump" => {
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
            },*/
            "undo" => {
                self.contents.undo();

                self.cursor.borrow_mut().number_line_size = self.contents.get_line_count();

                //todo: move the cursor somewhere
                //self.cursor.borrow_mut().set_cursor(CursorMove::Nothing, CursorMove::Amount(self.contents.get_line_count()), self, (0,0));
                
            },
            "redo" => {
                self.contents.redo();
                self.cursor.borrow_mut().number_line_size = self.contents.get_line_count();

            },
            "change_tab" => {
                if let Some(tab) = command_args.next() {
                    if let Ok(tab) = tab.parse::<usize>() {
                        self.sender.send(WindowMessage::NthTab(tab)).expect("Failed to send message");
                    }
                    else {
                        match tab {
                            "prev" => {
                                self.sender.send(WindowMessage::PreviousTab).expect("Failed to send message");
                            },
                            "next" => {
                                self.sender.send(WindowMessage::NextTab).expect("Failed to send message");
                            },
                            _ => {}
                        }
                    }
                }
            },
            "open_tab" => {
                self.sender.send(WindowMessage::OpenNewTab).expect("Failed to send message");
            },
            "open_tab_with_pane" => {
                self.sender.send(WindowMessage::OpenNewTabWithPane).expect("Failed to send message");
            },
            /*"paste" => {
                
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

            },*/

            _ => {}
        }
    }
}


impl TextBuffer for TextPane {
    fn save_buffer(&mut self) -> io::Result<()> {
        if let Some(file_name) = &self.file_name {
            let mut file = std::fs::File::create(file_name)?;
            file.write_all(self.contents.to_string().as_bytes())?;
        }
        Ok(())
    }

    fn open_file(&mut self, filename: PathBuf) -> std::io::Result<()> {

        let file_type = filename.extension().and_then(|s| s.to_str()).unwrap_or("txt");

        let file = File::from(filename);
        self.contents = Buffer::from(file);
        self.file_name = Some(filename);

        match file_type {
            "scm" => {
                let language = unsafe { tree_sitter_scheme() };
                let mut  parser = Parser::new();
                parser.set_language(language).unwrap();

                let tree = parser.parse(self.contents.to_string(), None).unwrap();

                let tree_sitter = TreeSitterInfo::new(parser, tree, "scheme".to_string());

                self.tree_sitter_info = Some(tree_sitter);

            },
            "rs" => {
                self.lsp_sender.send(LspControllerMessage::CreateClient("rust"
                                                                        .to_string()
                                                                        .into()))
                    .expect("Failed to send message");

                let lsp_client;

                loop {
                    match self.lsp_listener.try_recv() {
                        Ok(LspControllerMessage::ClientCreated(language_rcv)) => {
                            lsp_client = language_rcv;
                            break;
                        }
                        Ok(_) => {
                            continue;
                        }
                        Err(TryRecvError::Empty) => {
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => {
                            unreachable!();
                        }
                        
                    }
                }

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));

                let language = tree_sitter_rust::language();
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();

                let tree = parser.parse(self.contents.to_string(), None).unwrap();

                let tree_sitter = TreeSitterInfo::new(parser, tree, "rust".to_string());

                self.tree_sitter_info = Some(tree_sitter);

                let lsp_client = LspInfo::new(lsp_client);

                self.lsp_info = Some(lsp_client);
            }
            //todo: move h to C++ since there is no easy way of knowing which lang it is
            "c" | "h" => {
                let language = tree_sitter_c::language();
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();

                let tree = parser.parse(self.contents.to_string(), None).unwrap();

                let tree_sitter = TreeSitterInfo::new(parser, tree, "c".to_string());

                self.tree_sitter_info = Some(tree_sitter);

                self.lsp_sender.send(LspControllerMessage::CreateClient("c".to_string().into()))
                    .expect("Failed to send message");

                let lsp_client;

                loop {
                    match self.lsp_listener.try_recv() {
                        Ok(LspControllerMessage::ClientCreated(language_rcv)) => {
                            lsp_client = language_rcv;
                            break;
                        }
                        Ok(_) => {
                            continue;
                        }
                        Err(TryRecvError::Empty) => {
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => {
                            unreachable!();
                        }
                        
                    }
                }

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));

                let lsp_client = LspInfo::new(lsp_client);

                self.lsp_info = Some(lsp_client);
            }
            "cpp" | "hpp" => {
                let language = tree_sitter_cpp::language();
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();

                let tree = parser.parse(self.contents.to_string(), None).unwrap();

                let tree_sitter = TreeSitterInfo::new(parser, tree, "cpp".to_string());

                self.tree_sitter_info = Some(tree_sitter);

                self.lsp_sender.send(LspControllerMessage::CreateClient("cpp".to_string().into()))
                    .expect("Failed to send message");

                let lsp_client;

                loop {
                    match self.lsp_listener.try_recv() {
                        Ok(LspControllerMessage::ClientCreated(language_rcv)) => {
                            lsp_client = language_rcv;
                            break;
                        }
                        Ok(_) => {
                            continue;
                        }
                        Err(TryRecvError::Empty) => {
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => {
                            unreachable!();
                        }
                        
                    }
                }

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));

                let lsp_client = LspInfo::new(lsp_client);

                self.lsp_info = Some(lsp_client);
            }
            "py" => {
                let language = tree_sitter_python::language();
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();

                let tree = parser.parse(self.contents.to_string(), None).unwrap();

                let tree_sitter = TreeSitterInfo::new(parser, tree, "python".to_string());

                self.tree_sitter_info = Some(tree_sitter);

                self.lsp_sender.send(LspControllerMessage::CreateClient("python".to_string().into()))
                    .expect("Failed to send message");

                let lsp_client;

                loop {
                    match self.lsp_listener.try_recv() {
                        Ok(LspControllerMessage::ClientCreated(language_rcv)) => {
                            lsp_client = language_rcv;
                            break;
                        }
                        Ok(_) => {
                            continue;
                        }
                        Err(TryRecvError::Empty) => {
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => {
                            unreachable!();
                        }
                        
                    }
                }

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));

                let lsp_client = LspInfo::new(lsp_client);

                self.lsp_info = Some(lsp_client);
            }
            "lsp" => {
                let language = tree_sitter_commonlisp::language();
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();

                let tree = parser.parse(self.contents.to_string(), None).unwrap();

                let tree_sitter = TreeSitterInfo::new(parser, tree, "commonlisp".to_string());

                self.tree_sitter_info = Some(tree_sitter);
            }
            "swift" => {
                let language = tree_sitter_swift::language();
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();

                let tree = parser.parse(self.contents.to_string(), None).unwrap();

                let tree_sitter = TreeSitterInfo::new(parser, tree, "swift".to_string());

                self.tree_sitter_info = Some(tree_sitter);

                self.lsp_sender.send(LspControllerMessage::CreateClient("swift".to_string().into()))
                    .expect("Failed to send message");

                let lsp_client;

                loop {
                    match self.lsp_listener.try_recv() {
                        Ok(LspControllerMessage::ClientCreated(language_rcv)) => {
                            lsp_client = language_rcv;
                            break;
                        }
                        Ok(_) => {
                            continue;
                        }
                        Err(TryRecvError::Empty) => {
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => {
                            unreachable!();
                        }
                        
                    }
                }

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));

                let lsp_client = LspInfo::new(lsp_client);

                self.lsp_info = Some(lsp_client);
            }
            "go" => {
                let language = tree_sitter_go::language();
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();

                let tree = parser.parse(self.contents.to_string(), None).unwrap();

                let tree_sitter = TreeSitterInfo::new(parser, tree, "go".to_string());

                self.tree_sitter_info = Some(tree_sitter);

                self.lsp_sender.send(LspControllerMessage::CreateClient("go".to_string().into()))
                    .expect("Failed to send message");

                let lsp_client;

                loop {
                    match self.lsp_listener.try_recv() {
                        Ok(LspControllerMessage::ClientCreated(language_rcv)) => {
                            lsp_client = language_rcv;
                            break;
                        }
                        Ok(_) => {
                            continue;
                        }
                        Err(TryRecvError::Empty) => {
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => {
                            unreachable!();
                        }
                        
                    }
                }

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));

                let lsp_client = LspInfo::new(lsp_client);

                self.lsp_info = Some(lsp_client);
            }
            "sh" => {
                let language = tree_sitter_bash::language();
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();

                let tree = parser.parse(self.contents.to_string(), None).unwrap();

                let tree_sitter = TreeSitterInfo::new(parser, tree, "bash".to_string());

                self.tree_sitter_info = Some(tree_sitter);

                self.lsp_sender.send(LspControllerMessage::CreateClient("bash".to_string().into()))
                    .expect("Failed to send message");

                let lsp_client;

                loop {
                    match self.lsp_listener.try_recv() {
                        Ok(LspControllerMessage::ClientCreated(language_rcv)) => {
                            lsp_client = language_rcv;
                            break;
                        }
                        Ok(_) => {
                            continue;
                        }
                        Err(TryRecvError::Empty) => {
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => {
                            unreachable!();
                        }
                        
                    }
                }

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));

                let lsp_client = LspInfo::new(lsp_client);

                self.lsp_info = Some(lsp_client);
            }
            "js" => {
                let language = tree_sitter_javascript::language();
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();

                let tree = parser.parse(self.contents.to_string(), None).unwrap();

                let tree_sitter = TreeSitterInfo::new(parser, tree, "javascript".to_string());

                self.tree_sitter_info = Some(tree_sitter);
            }
            "cs" => {
                let language = tree_sitter_c_sharp::language();
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();

                let tree = parser.parse(self.contents.to_string(), None).unwrap();

                let tree_sitter = TreeSitterInfo::new(parser, tree, "c_sharp".to_string());

                self.tree_sitter_info = Some(tree_sitter);
            }
            "java" => {
                let language = tree_sitter_java::language();
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();

                let tree = parser.parse(self.contents.to_string(), None).unwrap();

                let tree_sitter = TreeSitterInfo::new(parser, tree, "java".to_string());

                self.tree_sitter_info = Some(tree_sitter);
            }
            "txt" | _ => {
                

            }


        }

        self.backup_buffer();

        Ok(())
    }

    fn backup_buffer(&mut self) {
        self.contents.add_new_rope();
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

    fn get_line_count(&self) -> usize {
        self.contents.get_line_count()
    }

    fn buffer_to_string(&self) -> String {
        self.contents.to_string()
    }

    fn get_row_len(&self, row: usize) -> Option<usize> {
        self.contents.line_len(row)
    }

    fn borrow_buffer(&self) -> &Buffer {
        &self.contents
    }

    fn borrow_mut_buffer(&mut self) -> &mut Buffer {
        &mut self.contents
    }
}



