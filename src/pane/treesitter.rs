use std::{sync::{mpsc::{Sender, Receiver}, Arc}, cell::RefCell, rc::Rc, path::PathBuf, collections::{HashMap, HashSet}, io::{self, Write}};

use crop::RopeSlice;
use crossterm::{event::KeyEvent, style::{Attribute, Color}};
use tree_sitter::{Parser, Tree, Point, Language, InputEdit};

use crate::{window::{Message, StyledChar}, cursor::{Cursor, Direction, CursorMove}, mode::{Mode, base::{Normal, Insert, Command},  PromptType}, buffer::Buffer, settings::{Settings, SyntaxHighlight, ColorScheme},  lsp::{ControllerMessage, LspNotification, lsp_utils::{Diagnostic, Diagnostics, CompletionList, TextEditType, LocationResponse}, LspResponse, LspRequest}};

use super::{text::{JumpTable, Waiting}, PaneMessage, Pane, PaneContainer, popup::PopUpPane};



pub struct TreesitterPane {
    parser: Parser,
    tree: Tree,
    lang: String,
    lsp_client: Option<(Sender<ControllerMessage>, Arc<Receiver<ControllerMessage>>)>,
    file_version: usize,
    lsp_diagnostics: Diagnostics,
    sent_diagnostics: HashSet<Diagnostic>,
    lsp_completion: Option<CompletionList>,
    lsp_location: Option<LocationResponse>,

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
    rainbow_delimiters: RefCell<Vec<(char, ColorScheme)>>,
}

impl TreesitterPane {
    pub fn new(settings: Rc<RefCell<Settings>>,
               sender: Sender<Message>,
               lang: Language,
               lang_string: &str,
               lsp: Option<(Sender<ControllerMessage>, Arc<Receiver<ControllerMessage>>)>)
               -> Self {
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

        let mut parser = Parser::new();

        parser.set_language(lang).unwrap();

        let tree = parser.parse("".as_bytes(), None).unwrap();
                

        
        Self {
            parser,
            tree,
            lsp_client: lsp,
            file_version: 0,
            lsp_diagnostics: Diagnostics::new(),
            sent_diagnostics: HashSet::new(),
            lsp_completion: None,
            lsp_location: None,
            lang: lang_string.to_string(),
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
            rainbow_delimiters: RefCell::new(Vec::new()),
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
                                    Waiting::Completion => {
                                        self.waiting = Waiting::None;
                                        let command = format!("insert {}", string);
                                        self.run_command(&command, container);
                                    },
                                    Waiting::None => {
                                    },
                                }
                            },
                            PaneMessage::Close => {
                                eprintln!("Closing treesitter");
                                self.run_command("q!", container)
                            },
                        }
                    },
                    Err(_) => {},
                }
            }
        }
    }

    fn generate_uri(& self) -> String {
        let working_dir = std::env::current_dir().unwrap();
        match &self.file_name {
            None => format!("untitled://{}", working_dir.display()),
            Some(file_name) => {
                let uri = format!("file://{}/{}", working_dir.display(), file_name.display());

                uri
            },
        }
    }

    fn read_lsp_messages(&mut self) {
        match self.lsp_client.as_ref() {
            None => {},
            Some((sender, receiver)) => {
                let mut other_uri_count = 0;
                loop {
                    match receiver.try_recv() {
                        Ok(ControllerMessage::Response(resp)) => {
                            match resp {
                                LspResponse::PublishDiagnostics(diags) => {
                                    if diags.uri == self.generate_uri() {
                                        self.lsp_diagnostics.merge(diags);
                                    }
                                    else {
                                        if other_uri_count == 4 {
                                            break;
                                        }
                                        other_uri_count += 1;
                                        sender.send(ControllerMessage::Resend(
                                            self.lang.clone().into(),
                                            LspResponse::PublishDiagnostics(diags)
                                        )).unwrap();
                                    }
                                },
                                LspResponse::Completion(completions) => {
                                    self.lsp_completion = Some(completions);
                                },
                                LspResponse::Location(location) => {
                                    self.lsp_location = Some(location);
                                },
                            }

                        },
                        Ok(_) => {
                        },
                        Err(_) => {
                            break;
                        },
                    }
                }
            },
        }
    }

    /// This function is to create an informational popup that will display
    /// the diagnostics for the current cursor position.
    /// Therefore, it should get called whenever something might have changed
    /// the diagnostics for the current cursor position.
    fn open_info(&mut self, container: &PaneContainer) {

        if self.lsp_completion.is_some() {
            return;
        }

        match &self.popup_channels {
            Some((send,_)) => {
                match self.waiting {
                    Waiting::None => {
                        eprintln!("Sending close");

                        match send.send(PaneMessage::Close) {
                            Ok(_) => {},
                            Err(_) => {},
                        }

                        self.popup_channels = None;
                    },
                    _ => {},
                }
            },
            None => {},
        }


        
        let cursor = self.cursor.borrow().get_cursor();

        let diagnostic = self.lsp_diagnostics.get_diagnostic(cursor.1, cursor.0);
        match diagnostic {
            Some(diagnostic) => {

                if self.sent_diagnostics.contains(diagnostic) {
                    self.sent_diagnostics.remove(diagnostic);
                    return;
                }
                
                let (send, recv) = std::sync::mpsc::channel();
                let (send2, recv2) = std::sync::mpsc::channel();

                let prompt = match &diagnostic.code {
                    Some(code) => {
                        let split = code.split('_');

                        let mut words = Vec::new();
                        for word in split {
                            let mut chars = word.chars();
                            match chars.next() {
                                None => {},
                                Some(c) => {
                                    let mut word = String::new();
                                    word.push(c.to_ascii_uppercase());
                                    for c in chars {
                                        word.push(c);
                                    }
                                    words.push(word);
                                },
                            }
                        }

                        let prompt = words.join(" ");
                        

                        vec![prompt]
                    },
                    None => Vec::new(),
                };

                let mut max = 0;

                let body = diagnostic.message.split("\n").map(|s| if s.len() == 0 {
                    None
                } else {
                    if s.len() > max {
                        max = s.len();
                    }
                    eprintln!("{}", s);
                    Some(s.to_string())
                }).collect::<Vec<Option<String>>>();

                
                let size = (max + 1, body.len() + 2 + prompt.len());
                

                let pane = PopUpPane::new_info(self.settings.clone(),
                                               prompt,
                                               self.sender.clone(),
                                               send,
                                               recv2,
                                               body,
                                               false);
                let pane = Rc::new(RefCell::new(pane));

                let (_, (x2, y2)) = container.get_corners();
                let (x, y) = container.get_size();

                let pos = self.cursor.borrow().get_real_cursor();
                eprintln!("pos: {:?}", pos);

                let max_size = container.get_size();

                let mut container = PaneContainer::new(max_size, size, pane, self.settings.clone());
                container.set_position(pos);
                container.set_size(size);

                self.sender.send(Message::CreatePopup(container, false)).expect("Failed to send message");
                self.waiting = Waiting::None;

                self.popup_channels = Some((send2, recv));
                self.sent_diagnostics.insert(diagnostic.clone());
            },
            None => {
                match &self.popup_channels {
                    Some((send,_)) => {
                        match self.waiting {
                            Waiting::None => {
                                eprintln!("Sending close");

                                match send.send(PaneMessage::Close) {
                                    Ok(_) => {},
                                    Err(_) => {},
                                }
                                self.sent_diagnostics.clear();
                                self.popup_channels = None;
                            },
                            _ => {},
                        }
                    },
                    None => {},
                }

            },
        }
            
        
    }

    fn insert_str_at(&mut self, pos: (usize, usize), s: &str) {
        self.set_changed(true);

        let start_byte;
        let new_end_byte;
        
        let byte_pos = self.get_byte_offset_pos(pos);
        if self.contents.get_char_count() == 0 {
            self.contents.insert(0, s);
            start_byte = 0;
            new_end_byte = self.contents.get_byte_count();
        }
        else {
            let byte_pos = match byte_pos {
                None => self.contents.get_byte_count(),
                Some(byte_pos) => byte_pos,
            };
            self.contents.insert(byte_pos, s);
            start_byte = byte_pos;
            new_end_byte = self.contents.get_byte_count();
        }

        let (x, y) = pos;

        let new_char_len = self.contents.get_char_count();

        let insert_char_count = s.chars().count();


        let edit = InputEdit {
            start_byte,
            old_end_byte: start_byte,
            new_end_byte,
            start_position: Point::new(y, x),
            old_end_position: Point::new(y, x + insert_char_count),
            new_end_position: Point::new(y, new_char_len),
        };

        self.tree.edit(&edit);
        self.tree = self.parser.parse(&self.contents.to_string(), Some(&self.tree)).unwrap();
        
    }

    fn get_byte_offset_pos(&self, (x, y): (usize, usize)) -> Option<usize> {

        self.contents.get_byte_offset(x, y)
    }


}


impl Pane for TreesitterPane {
    fn draw_row(&self, mut index: usize, container: &super::PaneContainer, output: &mut Vec<Option<crate::window::StyledChar>>) {

        let mut cols = container.get_size().0;

        let ((x1, y1), _) = container.get_corners();


        if self.settings.borrow().editor_settings.border {

            let color_settings = &self.settings.borrow().colors.ui;
            
            if index == 0 && y1 != 0 {
                let string = "-".repeat(cols);

                for c in string.chars() {
                    output.push(Some(StyledChar::new(c, color_settings.clone())));
                }
                
                //output.push_str(apply_colors!("-".repeat(cols), color_settings));
                return;
            }
            else {
                index = index;
            }

            if x1 != 0 {
                let string = "|".to_string();

                for c in string.chars() {
                    output.push(Some(StyledChar::new(c, color_settings.clone())));
                }
                
                cols = cols.saturating_sub(1);
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
                    let string = format!("{:width$}", real_row + 1, width = num_width);

                    for c in string.chars() {
                        output.push(Some(StyledChar::new(c, color_settings.clone())));
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
                        output.push(Some(StyledChar::new(c, color_settings.clone())));
                    }
                }
                else if real_row + 1 <= number_of_lines {
                    let string = format!("{:width$}",
                                            ((real_row) as isize - (self.cursor.borrow().get_cursor().1 as isize)).abs() as usize,
                                            width = num_width);

                    for c in string.chars() {
                        output.push(Some(StyledChar::new(c, color_settings.clone())));
                    }
                }
            }

        }

        self.cursor.borrow_mut().number_line_size = num_width;

        let color_settings = &self.settings.borrow().colors.pane;
        let syntax_highlighting = self.settings.borrow().colors.treesitter.clone();
        let default = HashMap::new();
        let syntax_highlighting = syntax_highlighting.get(&self.lang).unwrap_or(&default);


        if let Some(row) = self.get_row(real_row, col_offset, cols) {
            let mut count = 0;
            
            row.chars().for_each(|c| if count != (cols - num_width) {
                let point1 = Point::new(real_row, count);
                let point2 = Point::new(real_row, count + 1);
                let node = self.tree.root_node().descendant_for_point_range(point1, point2).unwrap();
                let parent_node = node.parent();
                
                match c {
                    '\t' => {
                        let string = " ".repeat(self.settings.borrow().editor_settings.tab_size);
                        
                        count += self.settings.borrow().editor_settings.tab_size;

                        for c in string.chars() {
                            output.push(Some(StyledChar::new(c, color_settings.clone())));
                        }
                    },
                    '\n' => {
                        count += 1;
                        let string = " ".to_string();

                        for c in string.chars() {
                            output.push(Some(StyledChar::new(c, color_settings.clone())));
                        }
                    },
                    ' ' => {
                        count += 1;
                        let string = " ".to_string();

                        for c in string.chars() {
                            output.push(Some(StyledChar::new(c, color_settings.clone())));
                        }
                    },
                    c => {
                        count += 1;
                        let string = c.to_string();

                        for c in string.chars() {

                            let color_settings = if let Some(settings) = syntax_highlighting.get(&node.kind().to_string()) {
                                match settings {
                                    SyntaxHighlight::Child(settings) => {
                                        settings
                                    }
                                    SyntaxHighlight::ChildExclude(settings, exclude) => {
                                        if exclude.contains(&c) {
                                            color_settings
                                        }
                                        else {
                                            settings
                                        }
                                    }
                                    _ => {
                                        color_settings
                                    }
                                }
                            }
                            else if let Some(mut parent) = parent_node {
                                let mut colors = color_settings;

                                if let Some(settings) = syntax_highlighting.get(&parent.kind().to_string()) {
                                    match settings {
                                        SyntaxHighlight::Child(settings) => {
                                            colors = settings;
                                        }
                                        SyntaxHighlight::ChildExclude(settings, exclude) => {
                                            if exclude.contains(&c) {
                                                colors = color_settings;
                                            }
                                            else {
                                                colors = settings;
                                            }
                                        }
                                        SyntaxHighlight::Parent(color_map) => {
                                            let mut cursor = parent.walk();

                                            let mut index = 0;

                                            for child_node in parent.children(&mut cursor) {
                                                if child_node.id() == node.id() {
                                                    break;
                                                }
                                                index += 1;
                                            }

                                            if let Some(field) = parent.field_name_for_child(index) {
                                                if let Some(highlight) = color_map.get(field) {
                                                    colors = match highlight {
                                                        SyntaxHighlight::Child(settings) => settings,
                                                        SyntaxHighlight::ChildExclude(settings, exclude) => {
                                                            if exclude.contains(&c) {
                                                                color_settings
                                                            }
                                                            else {
                                                                settings
                                                            }
                                                        }
                                                        _ => {
                                                            color_settings
                                                        }
                                                    };
                                                }
                                            }
                                        },
                                        SyntaxHighlight::ParentExclude(color_map, exclude) => {
                                            if exclude.contains(&c) {
                                                colors = color_settings;
                                            }
                                            else {
                                                let mut cursor = parent.walk();

                                                let mut index = 0;

                                                for child_node in parent.children(&mut cursor) {
                                                    if child_node.id() == node.id() {
                                                        break;
                                                    }
                                                    index += 1;
                                                }

                                                if let Some(field) = parent.field_name_for_child(index) {
                                                    if let Some(highlight) = color_map.get(field) {
                                                        colors = match highlight {
                                                            SyntaxHighlight::Child(settings) => settings,
                                                            SyntaxHighlight::ChildExclude(settings, exclude) => {
                                                                if exclude.contains(&c) {
                                                                    color_settings
                                                                }
                                                                else {
                                                                    settings
                                                                }
                                                            }
                                                            _ => {
                                                                color_settings
                                                            }
                                                        };
                                                    }
                                                }
                                            }
                                        },
                                        SyntaxHighlight::GrandParent(grandparents) => {
                                            if let Some(highlight) = grandparents.get(&parent.kind().to_string()) {
                                                match highlight {
                                                    SyntaxHighlight::Child(ref settings) => {
                                                        colors = settings;
                                                    }
                                                    SyntaxHighlight::ChildExclude(ref settings, exclude) => {
                                                        if exclude.contains(&c) {
                                                            colors = color_settings;
                                                        }
                                                        else {
                                                            colors = settings;
                                                        }
                                                    }
                                                    SyntaxHighlight::Parent(color_map) => {
                                                        let mut cursor = parent.walk();

                                                        let mut index = 0;

                                                        for child_node in parent.children(&mut cursor) {
                                                            if child_node.id() == node.id() {
                                                                break;
                                                            }
                                                            index += 1;
                                                        }

                                                        if let Some(field) = parent.field_name_for_child(index) {
                                                            if let Some(highlight) = color_map.get(field) {
                                                                colors = match highlight {
                                                                    SyntaxHighlight::Child(settings) => settings,
                                                                    SyntaxHighlight::ChildExclude(settings, exclude) => {
                                                                        if exclude.contains(&c) {
                                                                            color_settings
                                                                        }
                                                                        else {
                                                                            settings
                                                                        }
                                                                    }
                                                                    _ => {
                                                                        color_settings
                                                                    }
                                                                };
                                                            }
                                                        }
                                                    },
                                                    SyntaxHighlight::ParentExclude(color_map, exclude) => {
                                                        if exclude.contains(&c) {
                                                            colors = color_settings;
                                                        }
                                                        else {
                                                            let mut cursor = parent.walk();

                                                            let mut index = 0;

                                                            for child_node in parent.children(&mut cursor) {
                                                                if child_node.id() == node.id() {
                                                                    break;
                                                                }
                                                                index += 1;
                                                            }

                                                            if let Some(field) = parent.field_name_for_child(index) {
                                                                if let Some(highlight) = color_map.get(field) {
                                                                    colors = match highlight {
                                                                        SyntaxHighlight::Child(settings) => settings,
                                                                        SyntaxHighlight::ChildExclude(settings, exclude) => {
                                                                            if exclude.contains(&c) {
                                                                                color_settings
                                                                            }
                                                                            else {
                                                                                settings
                                                                            }
                                                                        }
                                                                        _ => {
                                                                            color_settings
                                                                        }
                                                                    };
                                                                }
                                                            }
                                                        }
                                                    },
                                                    _ => {
                                                        colors = color_settings;
                                                    }
                                                }
                                            }
                                            else {
                                                let old_parent = parent;
                                                while let Some(new_parent) = parent.parent() {
                                                    parent = new_parent;
                                                    if let Some(highlight) = grandparents.get(&parent.kind().to_string()) {

                                                        match highlight {
                                                            SyntaxHighlight::Child(ref settings) => {
                                                                colors = settings;
                                                            }
                                                            SyntaxHighlight::ChildExclude(ref settings, exclude) => {
                                                                if exclude.contains(&c) {
                                                                    colors = color_settings;
                                                                }
                                                                else {
                                                                    colors = settings;
                                                                }
                                                            }
                                                            SyntaxHighlight::Parent(color_map) => {
                                                                let mut cursor = old_parent.walk();

                                                                let mut index = 0;

                                                                for child_node in old_parent.children(&mut cursor) {
                                                                    if child_node.id() == node.id() {
                                                                        break;
                                                                    }
                                                                    index += 1;
                                                                }

                                                                if let Some(field) = old_parent.field_name_for_child(index) {
                                                                    if let Some(highlight) = color_map.get(field) {
                                                                        colors = match highlight {
                                                                            SyntaxHighlight::Child(settings) => settings,
                                                                            SyntaxHighlight::ChildExclude(settings, exclude) => {
                                                                                if exclude.contains(&c) {
                                                                                    color_settings
                                                                                }
                                                                                else {
                                                                                    settings
                                                                                }
                                                                            }
                                                                            _ => {
                                                                                color_settings
                                                                            }
                                                                        };
                                                                    }
                                                                }
                                                            },
                                                            SyntaxHighlight::ParentExclude(color_map, exclude) => {
                                                                if exclude.contains(&c) {
                                                                    colors = color_settings;
                                                                }
                                                                else {
                                                                    let mut cursor = parent.walk();

                                                                    let mut index = 0;

                                                                    for child_node in old_parent.children(&mut cursor) {
                                                                        if child_node.id() == node.id() {
                                                                            break;
                                                                        }
                                                                        index += 1;
                                                                    }

                                                                    if let Some(field) = old_parent.field_name_for_child(index) {
                                                                        if let Some(highlight) = color_map.get(field) {
                                                                            colors = match highlight {
                                                                                SyntaxHighlight::Child(settings) => settings,
                                                                                SyntaxHighlight::ChildExclude(settings, exclude) => {
                                                                                    if exclude.contains(&c) {
                                                                                        color_settings
                                                                                    }
                                                                                    else {
                                                                                        settings
                                                                                    }
                                                                                }
                                                                                _ => {
                                                                                    color_settings
                                                                                }
                                                                            };
                                                                        }
                                                                    }
                                                                }
                                                            },
                                                            _ => {
                                                                colors = color_settings;
                                                            }
                                                        }
                                                        break;
                                                    }

                                                }

                                            }

                                        }

                                    }


                                }


                                
                                colors
                            }
                            else {
                                color_settings
                            };

                            

                            
                            match c {
                                '(' | ')' | '{' | '}' | '[' | ']' | '<' | '>' => {
                                    if self.settings.borrow().editor_settings.rainbow_delimiters {
                                        let colors = &self.settings.borrow().colors.rainbow_delimiters;
                                        match c {
                                            '(' => {
                                                let index = self.rainbow_delimiters.borrow().len() % colors.len();
                                                self.rainbow_delimiters.borrow_mut().push((c, colors[index].clone()));
                                                output.push(Some(StyledChar::new(c, colors[index].clone())));
                                            },
                                            '{' => {
                                                let index = self.rainbow_delimiters.borrow().len() % colors.len();
                                                self.rainbow_delimiters.borrow_mut().push((c, colors[index].clone()));
                                                output.push(Some(StyledChar::new(c, colors[index].clone())));
                                            },
                                            '[' => {
                                                let index = self.rainbow_delimiters.borrow().len() % colors.len();
                                                self.rainbow_delimiters.borrow_mut().push((c, colors[index].clone()));
                                                output.push(Some(StyledChar::new(c, colors[index].clone())));
                                            },
                                            ')' => {
                                                let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                                output.push(Some(StyledChar::new(c, color)));
                                            },
                                            '}' => {
                                                let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                                output.push(Some(StyledChar::new(c, color)));
                                            },
                                            ']' => {
                                                let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                                output.push(Some(StyledChar::new(c, color)));
                                            },
                                            '<' => {
                                                if let Some(parent) = parent_node {

                                                    match parent.kind() {
                                                        "type_arguments" | "system_lib_string" => {
                                                            let index = self.rainbow_delimiters.borrow().len() % colors.len();
                                                            self.rainbow_delimiters.borrow_mut().push((c, colors[index].clone()));
                                                            output.push(Some(StyledChar::new(c, colors[index].clone())));
                                                        },
                                                        _ => {
                                                            match node.kind() {
                                                                "system_lib_string" => {
                                                                    let index = self.rainbow_delimiters.borrow().len() % colors.len();
                                                                    self.rainbow_delimiters.borrow_mut().push((c, colors[index].clone()));
                                                                    output.push(Some(StyledChar::new(c, colors[index].clone())));
                                                                },
                                                                _ => {
                                                                    output.push(Some(StyledChar::new(c, color_settings.clone())));
                                                                },
                                                            }
                                                        },
                                                    }
                                                }
                                                else {
                                                    match node.kind() {
                                                        "system_lib_string" => {
                                                            let index = self.rainbow_delimiters.borrow().len() % colors.len();
                                                            self.rainbow_delimiters.borrow_mut().push((c, colors[index].clone()));
                                                            output.push(Some(StyledChar::new(c, colors[index].clone())));
                                                        },
                                                        _ => {
                                                            output.push(Some(StyledChar::new(c, color_settings.clone())));
                                                        },
                                                    }
                                                }
                                            },
                                            '>' => {
                                                if let Some(parent) = parent_node {

                                                    match parent.kind() {
                                                        "type_arguments" | "system_lib_string" => {
                                                            let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                                            output.push(Some(StyledChar::new(c, color)));
                                                        },
                                                        _ => {
                                                            match node.kind() {
                                                                "system_lib_string" => {
                                                                    let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                                                    output.push(Some(StyledChar::new(c, color)));
                                                                },
                                                                _ => {
                                                                    output.push(Some(StyledChar::new(c, color_settings.clone())));
                                                                },
                                                            }
                                                        },
                                                    }
                                                }
                                                else {
                                                    match node.kind() {
                                                        "system_lib_string" => {
                                                            let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                                            output.push(Some(StyledChar::new(c, color)));
                                                        },
                                                        _ => {
                                                            output.push(Some(StyledChar::new(c, color_settings.clone())));
                                                        },
                                                    }
                                                }
                                            },
                                            _ => {
                                            },
                                        }
                                    }
                                    else {
                                        output.push(Some(StyledChar::new(c, color_settings.clone())));
                                    }
                                },
                                _ => {

                                    let diagnostic = self.lsp_diagnostics.get_diagnostic(real_row, count);
                                    //eprintln!("Diagnostic: {:?}", diagnostic);
                                    match diagnostic {
                                        None => output.push(Some(StyledChar::new(c, color_settings.clone()))),
                                        Some(diagnostic) => {
                                            match diagnostic.severity {
                                                3 => {
                                                    let mut color_settings = color_settings.add_attribute(Attribute::Undercurled);
                                                    color_settings.underline_color = Color::DarkRed;
                                                    output.push(Some(StyledChar::new(c, color_settings)));
                                                },
                                                2 => {
                                                    let mut color_settings = color_settings.add_attribute(Attribute::Undercurled);
                                                    color_settings.underline_color = Color::DarkYellow;
                                                    output.push(Some(StyledChar::new(c, color_settings)));
                                                },
                                                1 | _ => {
                                                    let mut color_settings = color_settings.add_attribute(Attribute::Undercurled);
                                                    color_settings.underline_color = Color::Yellow;
                                                    output.push(Some(StyledChar::new(c, color_settings)));
                                                },
                                            }
                                        }
                                        
                                    }
                                    
                                    //output.push(Some(StyledChar::new(c, color_settings.clone())));
                                }
                            }
                        }
                    },
                }
            }
                                 else {
            });

            let string = " ".repeat(cols.saturating_sub(count + num_width));

            for c in string.chars() {
                output.push(Some(StyledChar::new(c, color_settings.clone())));
            }
        }
        else if real_row >= number_of_lines {
            let string = " ".repeat(cols);

            for c in string.chars() {
                output.push(Some(StyledChar::new(c, color_settings.clone())));
            }
        }
        else {
            let string = " ".repeat(cols.saturating_sub(num_width));

            for c in string.chars() {
                output.push(Some(StyledChar::new(c, color_settings.clone())));
            }
        }
    }

    fn refresh(&mut self, container: &mut PaneContainer) {
        self.mode.borrow_mut().refresh();
        self.check_messages(container);
        if self.rainbow_delimiters.borrow().len() > 0 {
            self.rainbow_delimiters.borrow_mut().clear();
        }

        self.read_lsp_messages();


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
        self.contents.set_settings(self.settings.clone());
        self.file_name = Some(PathBuf::from(filename));

        self.tree = self.parser.parse(self.contents.to_string().as_bytes(), None).unwrap();
        eprintln!("{}", self.contents.to_string());

        eprintln!("{}", self.tree.root_node().to_sexp());

        let uri = self.generate_uri();

        
        
        match &self.lsp_client {
            None => {},
            Some((sender, _)) => {
                sender.send(ControllerMessage::Notification(
                    self.lang.clone().into(),
                    LspNotification::Open(uri.clone().into(),
                                          self.contents.to_string().into()))
                ).expect("Failed to send message");

                sender.send(ControllerMessage::Request(
                    self.lang.clone().into(),
                    LspRequest::RequestDiagnostic(uri.into())
                )).expect("Failed to send message");
            },
        }


        
        Ok(())
    }

    fn process_keypress(&mut self, key: KeyEvent, container: &mut PaneContainer) -> io::Result<bool> {
        let mode = self.mode.clone();
        let result = mode.borrow_mut().process_keypress(key, self, container);
        
        self.open_info(container);
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

                let uri = self.generate_uri();
                match &self.lsp_client {
                    None => {},
                    Some((sender,_)) => {

                        sender.send(ControllerMessage::Notification(
                            self.lang.clone().into(),
                            LspNotification::Close(uri.into())
                        )).expect("Failed to send message");
                    },
                }
               
                
            },
            "w" => {

                self.file_version += 1;

                let uri = self.generate_uri();
                match &self.lsp_client {
                    None => {},
                    Some((sender, _)) => {
                        //TODO replace filename with URI

                        sender.send(ControllerMessage::Notification(
                            self.lang.clone().into(),
                            LspNotification::WillSave(uri.clone().into(), "manual".into())
                        )).expect("Failed to send message");

                    },
                }

                
                if let Some(file_name) = command_args.next() {
                    self.file_name = Some(PathBuf::from(file_name));
                }

                self.save_buffer().expect("Failed to save file");
                self.contents.add_new_rope();

                match &self.lsp_client {
                    None => {},
                    Some((sender, _)) => {

                        let text = self.contents.to_string();

                        sender.send(ControllerMessage::Notification(
                            self.lang.clone().into(),
                            LspNotification::Save(uri.into(), text.into())
                        )).expect("Failed to send message");
                    },
                }
            },
            "w!" => {

                self.file_version += 1;

                let uri = self.generate_uri();
                match &self.lsp_client {
                    None => {},
                    Some((sender, _)) => {
                        //TODO replace filename with URI

                        sender.send(ControllerMessage::Notification(
                            self.lang.clone().into(),
                            LspNotification::WillSave(uri.clone().into(), "manual".into())
                        )).expect("Failed to send message");

                    },
                }

                
                if let Some(file_name) = command_args.next() {
                    self.file_name = Some(PathBuf::from(file_name));
                }

                self.save_buffer().expect("Failed to save file");
                self.contents.add_new_rope();

                match &self.lsp_client {
                    None => {},
                    Some((sender, _)) => {

                        let text = self.contents.to_string();

                        sender.send(ControllerMessage::Notification(
                            self.lang.clone().into(),
                            LspNotification::Save(uri.into(), text.into())
                        )).expect("Failed to send message");
                    },
                }

            },
            "wq" => {


                self.file_version += 1;

                let uri = self.generate_uri();
                match &self.lsp_client {
                    None => {},
                    Some((sender, _)) => {
                        //TODO replace filename with URI

                        sender.send(ControllerMessage::Notification(
                            self.lang.clone().into(),
                            LspNotification::WillSave(uri.clone().into(), "manual".into())
                        )).expect("Failed to send message");

                    },
                }

                
                self.save_buffer().expect("Failed to save file");
                self.sender.send(Message::ClosePane(false, None)).unwrap();

                match &self.lsp_client {
                    None => {},
                    Some((sender, _)) => {

                        let text = self.contents.to_string();

                        sender.send(ControllerMessage::Notification(
                            self.lang.clone().into(),
                            LspNotification::Save(uri.clone().into(), text.into())
                        )).expect("Failed to send message");
                    },
                }


                match &self.lsp_client {
                    None => {},
                    Some((sender,_)) => {

                        sender.send(ControllerMessage::Notification(
                            self.lang.clone().into(),
                            LspNotification::Close(uri.into())
                        )).expect("Failed to send message");
                    },
                }
                
            },
            "q!" => {
                self.sender.send(Message::ClosePane(false, None)).unwrap();
                let uri = self.generate_uri();

                match &self.lsp_client {
                    None => {},
                    Some((sender,_)) => {

                        sender.send(ControllerMessage::Notification(
                            self.lang.clone().into(),
                            LspNotification::Close(uri.into())
                        )).expect("Failed to send message");
                    },
                }

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
                                    //eprintln!("New Cursor: {:?}", new_cursor);
                                    //eprintln!("Old Cursor: {:?}", *cursor);
                                    //eprintln!("Jumping to named jump");
                                    *cursor = new_cursor;
                                }

                            }

                        }

                    }

                    self.open_info(container);
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
                    self.sender.send(Message::OpenFile(file_name.to_string())).expect("Failed to send message");
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

                self.tree = self.parser.parse(&self.contents.to_string(),None).unwrap();//TODO: replace this with an incremental parse

            },
            "redo" => {
                self.contents.redo();
                self.cursor.borrow_mut().number_line_size = self.contents.get_line_count();

                self.tree = self.parser.parse(&self.contents.to_string(),None).unwrap();//TODO: replace this with an incremental parse
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
            "info" => {
                self.open_info(container);
            },
            "completion" => {
                match &self.lsp_client {
                    None => {},
                    Some((sender, _)) => {

                        match self.popup_channels.take() {
                            None => {},
                            Some((send, _)) => {
                                match send.send(PaneMessage::Close) {
                                    Ok(_) => {},
                                    Err(_) => {},
                                }
                            }
                        }


                        let uri = self.generate_uri();

                        let position = self.cursor.borrow().get_cursor();

                        sender.send(ControllerMessage::Request(
                             self.lang.clone().into(),
                            LspRequest::RequestCompletion(uri.into(), position, "invoked".into())
                        )).expect("Failed to send message");

                        eprintln!("Sent completion request");
                        while self.lsp_completion.is_none() {
                            self.read_lsp_messages();
                        }

                        let completion_list = match self.lsp_completion.take() {
                            None => return,
                            Some(list) => list,
                        };

                        let buttons = completion_list.generate_buttons(70);


                        
                        let (send, recv) = std::sync::mpsc::channel();
                        let (send2, recv2) = std::sync::mpsc::channel();

                        self.popup_channels = Some((send2, recv));

                        let prompt = Vec::new();

                        let size = (70, buttons.button_len().expect("Buttons were not buttons") + 2);

                        
                        let pane = PopUpPane::new_dropdown(
                            self.settings.clone(),
                            prompt,
                            self.sender.clone(),
                            send,
                            recv2,
                            buttons,
                            false,
                        );

                        let pane = Rc::new(RefCell::new(pane));

                        let pos = self.cursor.borrow().get_real_cursor();
                        

                        let max_size = container.get_size();

                        let mut container = PaneContainer::new(max_size, size, pane, self.settings.clone());


                        container.set_position(pos);
                        container.set_size(size);



                        self.sender.send(Message::CreatePopup(container, true)).expect("Failed to send message");
                        self.waiting = Waiting::Completion;

                        self.contents.add_new_rope();

                        self.lsp_completion = Some(completion_list);
                    },
                }
                
            },
            "insert" => {
                if let Some(index) = command_args.next() {
                    eprintln!("Inserting {}", index);

                    if let Some(index) = index.parse::<usize>().ok() {
                        match &self.lsp_client {
                            None => {},
                            Some((_, _)) => {
                                match self.lsp_completion {
                                    None => {},
                                    Some(ref mut list) => {
                                        eprintln!("Inserting {}", index);
                                        if let Some(completion) = list.get_completion(index) {
                                            eprintln!("Inserting {}", index);
                                            if let Some(text_edit) = completion.get_edit_text() {
                                                eprintln!("Inserting {}", index);
                                                match text_edit {
                                                    TextEditType::TextEdit(text_edit) => {
                                                        let (pos, _) = text_edit.get_range();

                                                        self.insert_str_at(pos, &text_edit.newText);
                                                        eprintln!("Inserting {} at {:?}", &text_edit.newText, pos);
                                                    },
                                                    TextEditType::InsertReplaceEdit(text_edit) => {
                                                        unimplemented!();
                                                    },
                                                }
                                            }
                                        }
                                    },
                                }
                                
                                self.lsp_completion = None;
                            },
                        }

                    }
                }

                self.lsp_completion = None;
            },
            "goto_declaration" | "goto_definition" |
            "goto_type_definition" | "goto_implementation" => {
                match &self.lsp_client {
                    None => {},
                    Some((sender, _)) => {
                        let uri = self.generate_uri();

                        let position = self.cursor.borrow().get_cursor();

                        let request = match command {
                            "goto_declaration" => LspRequest::GotoDeclaration(uri.clone().into(), position),
                            "goto_definition" => LspRequest::GotoDefinition(uri.clone().into(), position),
                            "goto_type_definition" => LspRequest::GotoTypeDefinition(uri.clone().into(), position),
                            "goto_implementation" => LspRequest::GotoImplementation(uri.clone().into(), position),
                            _ => unreachable!(),
                        };


                        sender.send(ControllerMessage::Request(
                             self.lang.clone().into(),
                            request
                        )).expect("Failed to send message");

                        while self.lsp_location.is_none() {
                            self.read_lsp_messages();
                        }

                        let lsp_location = self.lsp_location.take().expect("LSP location was none");

                        match lsp_location {
                            LocationResponse::Location(location) => {
                                eprintln!("Got location {:?}", location);
                                
                                if location.uri.as_str() == uri.as_str() {
                                    eprintln!("Jumping to {:?}", location.range);
                                    self.jump_table.add(*self.cursor.borrow());

                                    let ((x, y), _) = location.range.get_positions();

                                    let mut cursor = self.cursor.borrow_mut();

                                    cursor.set_cursor(CursorMove::Amount(x), CursorMove::Amount(y), self, (0,0));
                                }
                                else {
                                    //TODO: Open a new pane and set its cursor to the location
                                }
                            },
                            LocationResponse::Locations(locations) => {

                                if locations.len() == 1 {
                                    
                                    if locations[0].uri.as_str() == uri.as_str() {
                                        eprintln!("Jumping to {:?}", locations[0].range);
                                        self.jump_table.add(*self.cursor.borrow());

                                        let ((x, y), _) = locations[0].range.get_positions();

                                        let mut cursor = self.cursor.borrow_mut();

                                        cursor.set_cursor(CursorMove::Amount(x), CursorMove::Amount(y), self, (0,0));
                                    }
                                    else {
                                        //TODO: Open a new pane and set its cursor to the location
                                    }
                                }

                            },
                            LocationResponse::LocationLink(location_link) => {
                                eprintln!("Got location link {:?}", location_link);

                            },
                            LocationResponse::Null => {},
                            
                            
                        }
                        

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

        let start_byte;
        //let old_end_byte = self.contents.get_byte_count();
        let new_end_byte;
        
        let byte_pos = self.get_byte_offset();
        let c = c.to_string();
        if self.contents.get_char_count() == 0 {
            self.contents.insert_current(0, c);
            new_end_byte = self.contents.get_byte_count();
            start_byte = 0;
        }
        else {
            let byte_pos = match byte_pos {
                None => self.contents.get_byte_count(),
                Some(byte_pos) => byte_pos,
            };

            self.contents.insert_current(byte_pos, c);

            new_end_byte = self.contents.get_byte_count();
            start_byte = byte_pos;
        }
        let (x, y) = self.cursor.borrow().get_cursor();

        let new_char_len = self.contents.get_char_count();
        
        let edit = InputEdit {
            start_byte,
            old_end_byte: start_byte,
            new_end_byte,
            start_position: Point::new(y, x),
            old_end_position: Point::new(y, x + 1),
            new_end_position: Point::new(y, new_char_len),
        };

        self.tree.edit(&edit);
        self.tree = self.parser.parse(&self.contents.to_string(), Some(&self.tree)).unwrap();

        self.file_version += 1;

        match &self.lsp_client {
            None => {},
            Some((sender, _)) => {
                let message = ControllerMessage::Notification(
                    self.lang.clone().into(),
                    LspNotification::ChangeText(
                        self.generate_uri().into(),
                        self.file_version,
                        self.contents.to_string().into(),
                    )
                );

                sender.send(message).expect("Failed to send message");
                        
            },
        }

    }

    fn insert_str(&mut self, s: &str) {
        self.set_changed(true);

        let start_byte;
        let new_end_byte;
        
        let byte_pos = self.get_byte_offset();
        if self.contents.get_char_count() == 0 {
            self.contents.insert(0, s);
            start_byte = 0;
            new_end_byte = self.contents.get_byte_count();
        }
        else {
            let byte_pos = match byte_pos {
                None => self.contents.get_byte_count(),
                Some(byte_pos) => byte_pos,
            };
            self.contents.insert(byte_pos, s);
            start_byte = byte_pos;
            new_end_byte = self.contents.get_byte_count();
        }

        let (x, y) = self.cursor.borrow().get_cursor();

        let new_char_len = self.contents.get_char_count();

        let insert_char_count = s.chars().count();


        let edit = InputEdit {
            start_byte,
            old_end_byte: start_byte,
            new_end_byte,
            start_position: Point::new(y, x),
            old_end_position: Point::new(y, x + insert_char_count),
            new_end_position: Point::new(y, new_char_len),
        };

        self.tree.edit(&edit);
        self.tree = self.parser.parse(&self.contents.to_string(), Some(&self.tree)).unwrap();

        self.file_version += 1;
        
        match &self.lsp_client {
            None => {},
            Some((sender, _)) => {
                let message = ControllerMessage::Notification(
                    self.lang.clone().into(),
                    LspNotification::ChangeText(
                        self.generate_uri().into(),
                        self.file_version,
                        self.contents.to_string().into(),
                    )
                );

                sender.send(message).expect("Failed to send message");
                        
            },
        }
        
    }

    ///TODO: add check to make sure we have a valid byte range
    fn delete_char(&mut self) {
        self.set_changed(true);

        
        let byte_pos = self.get_byte_offset();

        let byte_pos = match byte_pos {
            None => return,
            Some(byte_pos) => byte_pos,
        };
        let start_byte = byte_pos;

        let old_end_byte = self.contents.get_byte_count();

        self.contents.delete(byte_pos..byte_pos.saturating_add(1));

        let new_end_byte = self.contents.get_byte_count();

        let (x, y) = self.cursor.borrow().get_cursor();

        let edit = InputEdit {
            start_byte,
            old_end_byte,
            new_end_byte,
            start_position: Point::new(y, x),
            old_end_position: Point::new(y, x - 1),
            new_end_position: Point::new(y, x),
        };

        self.tree.edit(&edit);

        self.tree = self.parser.parse(&self.contents.to_string(), Some(&self.tree)).unwrap();

        self.file_version += 1;
        
        match &self.lsp_client {
            None => {},
            Some((sender, _)) => {
                let message = ControllerMessage::Notification(
                    self.lang.clone().into(),
                    LspNotification::ChangeText(
                        self.generate_uri().into(),
                        self.file_version,
                        self.contents.to_string().into(),
                    )
                );

                sender.send(message).expect("Failed to send message");
                        
            },
        }

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

        let (x, y) = cursor.get_cursor();

        if go_up {
            cursor.move_cursor(Direction::Up, 1, self);
            cursor.set_cursor(CursorMove::ToEnd, CursorMove::Nothing, self, (0, 1));
        }
        else {
            cursor.move_cursor(Direction::Left, 1, self);
        }

        let start_byte = byte_pos.saturating_sub(1);
        let old_end_byte = self.contents.get_byte_count();
        

        self.contents.delete(byte_pos.saturating_sub(1)..byte_pos);

        let new_end_byte = self.contents.get_byte_count();

        let edit = InputEdit {
            start_byte,
            old_end_byte,
            new_end_byte,
            start_position: Point::new(y, x),
            old_end_position: Point::new(y, x.saturating_sub(1)),
            new_end_position: Point::new(y, x),
        };

        self.tree.edit(&edit);

        self.tree = self.parser.parse(&self.contents.to_string(), Some(&self.tree)).unwrap();

        self.file_version += 1;
        
        match &self.lsp_client {
            None => {},
            Some((sender, _)) => {
                let message = ControllerMessage::Notification(
                    self.lang.clone().into(),
                    LspNotification::ChangeText(
                        self.generate_uri().into(),
                        self.file_version,
                        self.contents.to_string().into(),
                    )
                );

                sender.send(message).expect("Failed to send message");
                        
            },
        }

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
