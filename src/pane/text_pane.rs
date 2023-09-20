use std::{path::PathBuf, sync::{mpsc::{Sender, Receiver, TryRecvError}, Arc}, collections::{HashSet, HashMap}, rc::Rc, cell::RefCell, io::{self, Write, BufReader, Read}, fs::File};

use crossterm::style::{Attribute, Color};
use tree_sitter::{Tree, Parser, Point, InputEdit};

use crate::{buffer::Buffer, lsp::{LspControllerMessage, lsp_utils::{Diagnostics, CompletionList, LocationResponse, Diagnostic, TextEditType}, LspNotification, LspRequest, LspResponse}, cursor::{Cursor, Direction, CursorMove}, mode::{Mode, base::{Normal, Insert, Command}, TextMode, PromptType, Promptable}, settings::{Settings, SyntaxHighlight, ColorScheme}, window::WindowMessage, treesitter::tree_sitter_scheme, window::{TextRow, StyledChar}, editor::{EditorMessage, RegisterType}};

use super::{Pane, TextBuffer, PaneMessage, PaneContainer, popup::PopUpPane};




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
    pub lang: String,
    pub lsp_client: (Sender<LspControllerMessage>, Arc<Receiver<LspControllerMessage>>),
    pub file_version: usize,
    pub lsp_diagnostics: Diagnostics,
    pub sent_diagnostics: HashSet<Diagnostic>,
    pub lsp_completion: Option<CompletionList>,
    pub lsp_location: Option<LocationResponse>,
}


impl LspInfo {
    pub fn new(lang: &str, lsp_client: (Sender<LspControllerMessage>, Arc<Receiver<LspControllerMessage>>)) -> Self {
        Self {
            lang: lang.to_owned(),
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
    mode: Rc<RefCell<dyn TextMode>>,
    modes: HashMap<String, Rc<RefCell<dyn TextMode>>>,
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

    rainbow_delimiters: RefCell<Vec<(char, ColorScheme)>>,


}


impl TextPane {
    pub fn new(settings: Rc<RefCell<Settings>>,
               window_sender: Sender<WindowMessage>,
               window_receiver: Rc<Receiver<WindowMessage>>,
               editor_sender: Sender<EditorMessage>,
               editor_receiver: Rc<Receiver<EditorMessage>>,
               lsp_sender: Sender<LspControllerMessage>,
               lsp_listener: Rc<Receiver<LspControllerMessage>>) -> Self {


        let mut modes: HashMap<String, Rc<RefCell<dyn TextMode>>> = HashMap::new();
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

            rainbow_delimiters: RefCell::new(Vec::new()),
        }
        
    }

    fn set_changed(&mut self, changed: bool) {
        self.text_changed = changed;
    }

    fn get_byte_offset(&self) -> Option<usize> {
        let (x, y) = self.cursor.borrow().get_cursor();

        self.contents.get_byte_offset(x, y)
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
        match self.lsp_info.take() {
            None => {},
            Some(mut lsp_info) => {

                let LspInfo { lang,
                              lsp_client: (sender, receiver),
                              lsp_diagnostics,
                              lsp_completion,
                              lsp_location,
                              .. } =&mut lsp_info;
                
                let mut other_uri_count = 0;
                loop {
                    match receiver.try_recv() {
                        Ok(LspControllerMessage::Response(resp)) => {
                            match resp {
                                LspResponse::PublishDiagnostics(diags) => {
                                    if diags.uri == self.generate_uri() {
                                        lsp_diagnostics.merge(diags);
                                    }
                                    else {
                                        if other_uri_count == 4 {
                                            break;
                                        }
                                        other_uri_count += 1;
                                        sender.send(LspControllerMessage::Resend(
                                            lang.clone().into(),
                                            LspResponse::PublishDiagnostics(diags)
                                        )).unwrap();
                                    }
                                },
                                LspResponse::Completion(completions) => {
                                    *lsp_completion = Some(completions);
                                },
                                LspResponse::Location(location) => {
                                    *lsp_location = Some(location);
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

                self.lsp_info = Some(lsp_info);
            },
        }
    }


    fn get_file_path(uri: &str) -> String {
        
        let chars = uri.chars();
        // Here we skip the `file://` part of the uri
        let chars = chars.skip(7);
        // We will need to skip the port number if there is one
        //TODO: Handle port numbers

        chars.collect()
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

        match &mut self.tree_sitter_info {
            None => {},
            Some(TreeSitterInfo { parser, tree, language }) => {
                let edit = InputEdit {
                    start_byte,
                    old_end_byte: start_byte,
                    new_end_byte,
                    start_position: Point::new(y, x),
                    old_end_position: Point::new(y, x + insert_char_count),
                    new_end_position: Point::new(y, new_char_len),
                };

                tree.edit(&edit);
                *tree = parser.parse(&self.contents.to_string(), Some(&tree)).unwrap();
            },

        }

        match self.lsp_info.take() {
            None => {},
            Some(mut lsp_info) => {

                let LspInfo { lang, lsp_client: (sender, _), file_version, .. } = &mut lsp_info;
                *file_version += 1;
                let message = LspControllerMessage::Notification(
                    lang.clone().into(),
                    LspNotification::ChangeText(
                        self.generate_uri().into(),
                        *file_version,
                        self.contents.to_string().into(),
                    )
                );

                sender.send(message).expect("Failed to send message");
                self.lsp_info = Some(lsp_info);
            }
        }


    }

    fn get_byte_offset_pos(&self, (x, y): (usize, usize)) -> Option<usize> {

        self.contents.get_byte_offset(x, y)
    }

    fn check_messages(&mut self, container: &mut PaneContainer) {
        if self.popup_channels.len() == 0 {
            return;
        }
        
        match self.popup_channels[0].1.try_recv() {
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
                            Waiting::Goto => {
                                self.waiting = Waiting::None;
                                let command = format!("goto {}", string);
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
    

    fn draw_row_treesitter(&self, mut index: usize, container: &PaneContainer, output: &mut TextRow) {

        let cols = container.get_size().0;
        //eprintln!("Cols: {}", cols);

        //eprintln!("Changed");

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
        //eprintln!("Col offset: {}", col_offset);

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
                        //eprintln!("Not Changed 1");
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
        let syntax_highlighting = self.settings.borrow().colors.treesitter.clone();
        let default = HashMap::new();
        let syntax_highlighting = syntax_highlighting.get(&self.tree_sitter_info.as_ref().unwrap().language).unwrap_or(&default);

        if !self.cursor.borrow().get_scrolled() {
            //eprintln!("Not Changed 2");
            for _ in 0..num_width {
                output.push(None);
            }
        }


        if !self.cursor.borrow().get_scrolled() {
            //eprintln!("Not Changed 3");
            for _ in 0..(cols - num_width) {
                output.push(None);
            }
        }
        else {

            if let Some(row) = self.contents.get_row(real_row, col_offset, cols - num_width) {
                //eprintln!("Row: {}", row);
                let mut count = 0;

                row.chars().for_each(|c| if count != (cols - num_width) {
                    let point1 = Point::new(real_row, count);
                    let point2 = Point::new(real_row, count + 1);
                    let node = self.tree_sitter_info.as_ref().unwrap().tree.root_node().descendant_for_point_range(point1, point2).unwrap();
                    let parent_node = node.parent();

                    match c {
                        '\t' => {
                            let string = " ".repeat(self.settings.borrow().editor_settings.tab_size);

                            count += self.settings.borrow().editor_settings.tab_size;

                            for c in string.chars() {
                                output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                            }
                        },
                        '\n' => {
                            count += 1;
                            let string = " ".to_string();

                            for c in string.chars() {
                                output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                            }
                        },
                        ' ' => {
                            count += 1;
                            let string = " ".to_string();

                            for c in string.chars() {
                                output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
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
                                                    output.push(Some(Some(StyledChar::new(c, colors[index].clone()))));
                                                },
                                                '{' => {
                                                    let index = self.rainbow_delimiters.borrow().len() % colors.len();
                                                    self.rainbow_delimiters.borrow_mut().push((c, colors[index].clone()));
                                                    output.push(Some(Some(StyledChar::new(c, colors[index].clone()))));
                                                },
                                                '[' => {
                                                    let index = self.rainbow_delimiters.borrow().len() % colors.len();
                                                    self.rainbow_delimiters.borrow_mut().push((c, colors[index].clone()));
                                                    output.push(Some(Some(StyledChar::new(c, colors[index].clone()))));
                                                },
                                                ')' => {
                                                    let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                                    output.push(Some(Some(StyledChar::new(c, color))));
                                                },
                                                '}' => {
                                                    let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                                    output.push(Some(Some(StyledChar::new(c, color))));
                                                },
                                                ']' => {
                                                    let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                                    output.push(Some(Some(StyledChar::new(c, color))));
                                                },
                                                '<' => {
                                                    if let Some(parent) = parent_node {

                                                        match parent.kind() {
                                                            "type_arguments" | "system_lib_string" => {
                                                                let index = self.rainbow_delimiters.borrow().len() % colors.len();
                                                                self.rainbow_delimiters.borrow_mut().push((c, colors[index].clone()));
                                                                output.push(Some(Some(StyledChar::new(c, colors[index].clone()))));
                                                            },
                                                            _ => {
                                                                match node.kind() {
                                                                    "system_lib_string" => {
                                                                        let index = self.rainbow_delimiters.borrow().len() % colors.len();
                                                                        self.rainbow_delimiters.borrow_mut().push((c, colors[index].clone()));
                                                                        output.push(Some(Some(StyledChar::new(c, colors[index].clone()))));
                                                                    },
                                                                    _ => {
                                                                        output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
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
                                                                output.push(Some(Some(StyledChar::new(c, colors[index].clone()))));
                                                            },
                                                            _ => {
                                                                output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                                                            },
                                                        }
                                                    }
                                                },
                                                '>' => {
                                                    if let Some(parent) = parent_node {

                                                        match parent.kind() {
                                                            "type_arguments" | "system_lib_string" => {
                                                                let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                                                output.push(Some(Some(StyledChar::new(c, color))));
                                                            },
                                                            _ => {
                                                                match node.kind() {
                                                                    "system_lib_string" => {
                                                                        let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                                                        output.push(Some(Some(StyledChar::new(c, color))));
                                                                    },
                                                                    _ => {
                                                                        output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                                                                    },
                                                                }
                                                            },
                                                        }
                                                    }
                                                    else {
                                                        match node.kind() {
                                                            "system_lib_string" => {
                                                                let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                                                output.push(Some(Some(StyledChar::new(c, color))));
                                                            },
                                                            _ => {
                                                                output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                                                            },
                                                        }
                                                    }
                                                },
                                                _ => {
                                                },
                                            }
                                        }
                                        else {
                                            output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                                        }
                                    },
                                    _ => {

                                        let diagnostic = self.lsp_info.as_ref().unwrap().lsp_diagnostics.get_diagnostic(real_row, count);
                                        //eprintln!("Diagnostic: {:?}", diagnostic);
                                        match diagnostic {
                                            None => output.push(Some(Some(StyledChar::new(c, color_settings.clone())))),
                                            Some(diagnostic) => {
                                                match diagnostic.severity {
                                                    3 => {
                                                        let mut color_settings = color_settings.add_attribute(Attribute::Undercurled);
                                                        color_settings.underline_color = Color::DarkRed;
                                                        output.push(Some(Some(StyledChar::new(c, color_settings))));
                                                    },
                                                    2 => {
                                                        let mut color_settings = color_settings.add_attribute(Attribute::Undercurled);
                                                        color_settings.underline_color = Color::DarkYellow;
                                                        output.push(Some(Some(StyledChar::new(c, color_settings))));
                                                    },
                                                    1 | _ => {
                                                        let mut color_settings = color_settings.add_attribute(Attribute::Undercurled);
                                                        color_settings.underline_color = Color::Yellow;
                                                        output.push(Some(Some(StyledChar::new(c, color_settings))));
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
    }


    fn draw_row_reg(&self, mut index: usize, container: &super::PaneContainer, output: &mut TextRow) {
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

        if let Some(row) = self.contents.get_row(real_row, col_offset, cols) {
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
                        match c {
                            '(' | ')' | '{' | '}' | '[' | ']'  => {
                                if self.settings.borrow().editor_settings.rainbow_delimiters {
                                    let colors = &self.settings.borrow().colors.rainbow_delimiters;
                                    match c {
                                        '(' => {
                                            let index = self.rainbow_delimiters.borrow().len() % colors.len();
                                            self.rainbow_delimiters.borrow_mut().push((c, colors[index].clone()));
                                            output.push(Some(Some(StyledChar::new(c, colors[index].clone()))));
                                        },
                                        '{' => {
                                            let index = self.rainbow_delimiters.borrow().len() % colors.len();
                                            self.rainbow_delimiters.borrow_mut().push((c, colors[index].clone()));
                                            output.push(Some(Some(StyledChar::new(c, colors[index].clone()))));
                                        },
                                        '[' => {
                                            let index = self.rainbow_delimiters.borrow().len() % colors.len();
                                            self.rainbow_delimiters.borrow_mut().push((c, colors[index].clone()));
                                            output.push(Some(Some(StyledChar::new(c, colors[index].clone()))));
                                        },
                                        ')' => {
                                            let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                            output.push(Some(Some(StyledChar::new(c, color))));
                                        },
                                        '}' => {
                                            let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                            output.push(Some(Some(StyledChar::new(c, color))));
                                        },
                                        ']' => {
                                            let (_, color) = self.rainbow_delimiters.borrow_mut().pop().unwrap_or((c, color_settings.clone()));
                                            output.push(Some(Some(StyledChar::new(c, color))));
                                        },
                                        _ => {
                                        },
                                    }
                                }
                                else {
                                    output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                                }
                            },
                            _ => {
                                output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                            }
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


}



impl Pane for TextPane {
    fn draw_row(&self, index: usize, container: &super::PaneContainer, output: &mut TextRow) {

        if self.tree_sitter_info.is_some() {
            self.draw_row_treesitter(index, container, output);
            return;
        } else {
            self.draw_row_reg(index, container, output);
        }
    }

    fn refresh(&mut self, container: &mut super::PaneContainer) {
        let cursor = self.cursor.clone();

        cursor.borrow_mut().scroll(container);
        self.rainbow_delimiters.borrow_mut().clear();

        self.check_messages(container);
        self.read_lsp_messages();
    }

    fn process_keypress(&mut self, key: crate::settings::Key, container: &mut super::PaneContainer) {
        let mode = self.mode.clone();
        mode.borrow_mut().process_keypress(key, self, container);

    }

    fn get_status(&self, container: &super::PaneContainer) -> Option<(String, String, String)> {
        Some(self.mode.borrow_mut().update_status(self, container))
    }


    fn reset(&mut self) {
        self.cursor.borrow_mut().reset_move();
        self.cursor.borrow_mut().reset_scrolled();
    }

    fn changed(&mut self) {
        self.cursor.borrow_mut().set_moved();
        self.cursor.borrow_mut().set_scrolled();
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

    fn run_command(&mut self, command: &str, container: &mut PaneContainer) {
        let mut command_args = command.split_whitespace();
        let command = command_args.next().unwrap_or("");
        match command {
            "q" => {
                if self.text_changed {
                } else {
                    self.window_sender.send(WindowMessage::ClosePane(false, None)).unwrap();
                }


                match &self.lsp_info {
                    None => {},
                    Some(LspInfo { lang,
                                   lsp_client: (sender, _),
                                   ..}) => {

                        let uri = self.generate_uri();

                        sender.send(LspControllerMessage::Notification(
                            lang.clone().into(),
                            LspNotification::Close(uri.into())
                        )).expect("Failed to send message");

                    },
                }

               
                
            },
            "w" => {

                match self.lsp_info.take() {
                    None => {},
                    Some(mut lsp_info) => {

                        let LspInfo { lang, lsp_client: (sender, _), file_version, .. } = &mut lsp_info;
                        *file_version += 1;
                        let uri = self.generate_uri();

                        sender.send(LspControllerMessage::Notification(
                            lang.clone().into(),
                            LspNotification::WillSave(uri.into(), "manual".into())
                        )).expect("Failed to send message");

                        self.lsp_info = Some(lsp_info);

                    },
                }

                
                if let Some(file_name) = command_args.next() {
                    self.file_name = Some(PathBuf::from(file_name));
                }

                self.save_buffer().expect("Failed to save file");
                self.contents.add_new_rope();

                match self.lsp_info.take() {
                    None => {},
                    Some(mut lsp_info) => {

                        let LspInfo {lang, lsp_client: (sender, _),  .. } = &mut lsp_info;

                        let uri = self.generate_uri();
                        let text = self.contents.to_string();

                        sender.send(LspControllerMessage::Notification(
                            lang.clone().into(),
                            LspNotification::Save(uri.into(),text.into())
                        )).expect("Failed to send message");

                        self.lsp_info = Some(lsp_info);
                    },
                }


                self.text_changed = false;
            },
            "w!" => {

                match self.lsp_info.take() {
                    None => {},
                    Some(mut lsp_info) => {

                        let LspInfo { lang, lsp_client: (sender, _), file_version, .. } = &mut lsp_info;
                        *file_version += 1;
                        let uri = self.generate_uri();

                        sender.send(LspControllerMessage::Notification(
                            lang.clone().into(),
                            LspNotification::WillSave(uri.into(), "manual".into())
                        )).expect("Failed to send message");

                        self.lsp_info = Some(lsp_info);

                    },
                }

                
                if let Some(file_name) = command_args.next() {
                    self.file_name = Some(PathBuf::from(file_name));
                }

                self.save_buffer().expect("Failed to save file");
                self.contents.add_new_rope();

                match self.lsp_info.take() {
                    None => {},
                    Some(mut lsp_info) => {

                        let LspInfo {lang, lsp_client: (sender, _),  .. } = &mut lsp_info;

                        let uri = self.generate_uri();
                        let text = self.contents.to_string();

                        sender.send(LspControllerMessage::Notification(
                            lang.clone().into(),
                            LspNotification::Save(uri.into(),text.into())
                        )).expect("Failed to send message");

                        self.lsp_info = Some(lsp_info);
                    },
                }

                self.text_changed = false;

            },
            "wq" => {


                match self.lsp_info.take() {
                    None => {},
                    Some(mut lsp_info) => {

                        let LspInfo { lang, lsp_client: (sender, _), file_version, .. } = &mut lsp_info;
                        *file_version += 1;
                        let uri = self.generate_uri();

                        sender.send(LspControllerMessage::Notification(
                            lang.clone().into(),
                            LspNotification::WillSave(uri.into(), "manual".into())
                        )).expect("Failed to send message");

                        self.lsp_info = Some(lsp_info);

                    },
                }

                
                self.save_buffer().expect("Failed to save file");
                self.window_sender.send(WindowMessage::ClosePane(false, None)).unwrap();

                match self.lsp_info.take() {
                    None => {},
                    Some(mut lsp_info) => {

                        let LspInfo {lang, lsp_client: (sender, _),  .. } = &mut lsp_info;

                        let uri = self.generate_uri();
                        let text = self.contents.to_string();

                        sender.send(LspControllerMessage::Notification(
                            lang.clone().into(),
                            LspNotification::Save(uri.into(),text.into())
                        )).expect("Failed to send message");

                        self.lsp_info = Some(lsp_info);
                    },
                }


                match &self.lsp_info {
                    None => {},
                    Some(LspInfo { lang,
                                   lsp_client: (sender, _),
                                   ..}) => {

                        let uri = self.generate_uri();

                        sender.send(LspControllerMessage::Notification(
                            lang.clone().into(),
                            LspNotification::Close(uri.into())
                        )).expect("Failed to send message");

                    },
                }
                
            },
            "q!" => {
                self.window_sender.send(WindowMessage::ClosePane(false, None)).unwrap();
                match &self.lsp_info {
                    None => {},
                    Some(LspInfo { lang,
                                   lsp_client: (sender, _),
                                   ..}) => {

                        let uri = self.generate_uri();

                        sender.send(LspControllerMessage::Notification(
                            lang.clone().into(),
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

                    //self.open_info(container);
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
                self.window_sender.send(WindowMessage::HorizontalSplit).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "vertical_split" => {
                self.window_sender.send(WindowMessage::VerticalSplit).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "qa!" => {
                self.window_sender.send(WindowMessage::ForceQuitAll).expect("Failed to send message");
            },
            "pane_up" => {
                self.window_sender.send(WindowMessage::PaneUp).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "pane_down" => {
                self.window_sender.send(WindowMessage::PaneDown).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "pane_left" => {
                self.window_sender.send(WindowMessage::PaneLeft).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "pane_right" => {
                self.window_sender.send(WindowMessage::PaneRight).expect("Failed to send message");
                self.contents.add_new_rope();
            },
            "e" => {
                if let Some(file_name) = command_args.next() {
                    self.window_sender.send(WindowMessage::OpenFile(file_name.to_string(), None)).expect("Failed to send message");
                }
                self.contents.add_new_rope();
            },
            "prompt_jump" => {
                let (send, recv) = std::sync::mpsc::channel();
                let (send2, recv2) = std::sync::mpsc::channel();

                self.popup_channels = vec![(send2, recv)];

                let txt_prompt = PromptType::Text(String::new(), None, false);
                let prompt = vec!["Enter Jump".to_string(), "Target".to_string()];

                let pane = PopUpPane::new_prompt(
                    self.settings.clone(),
                    prompt,
                    self.window_sender.clone(),
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
                
                let mut container = PaneContainer::new(pane, self.settings.clone());


                container.set_position(pos);
                container.set_size((14, 5));



                self.window_sender.send(WindowMessage::CreatePopup(container, true)).expect("Failed to send message");
                self.waiting = Waiting::JumpTarget;

                self.contents.add_new_rope();
            },
            "prompt_set_jump" => {
                let (send, recv) = std::sync::mpsc::channel();
                let (send2, recv2) = std::sync::mpsc::channel();

                self.popup_channels = vec![(send2, recv)];

                let txt_prompt = PromptType::Text(String::new(), None, false);
                let prompt = vec!["Name the".to_string(), "Target".to_string()];

                let pane = PopUpPane::new_prompt(
                    self.settings.clone(),
                    prompt,
                    self.window_sender.clone(),
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
                
                let mut container = PaneContainer::new(pane, self.settings.clone());


                container.set_position(pos);
                container.set_size((14, 5));



                self.window_sender.send(WindowMessage::CreatePopup(container, true)).expect("Failed to send message");
                self.waiting = Waiting::JumpPosition;

                self.contents.add_new_rope();
            },
            "undo" => {
                self.contents.undo();

                self.cursor.borrow_mut().number_line_size = self.contents.get_line_count();

                self.cursor.borrow_mut().set_cursor(CursorMove::Nothing, CursorMove::Amount(self.contents.get_line_count()), self, (0,0));

                match self.tree_sitter_info.as_mut() {
                    Some(tree_sitter_info) => {
                        tree_sitter_info.tree = tree_sitter_info.parser.parse(&self.contents.to_string(),None).unwrap();//TODO: replace this with an incremental parse
                    },
                    None => {}
                }

                //self.tree = self.parser.parse(&self.contents.to_string(),None).unwrap();//TODO: replace this with an incremental parse

            },
            "redo" => {
                self.contents.redo();
                self.cursor.borrow_mut().number_line_size = self.contents.get_line_count();

                match self.tree_sitter_info.as_mut() {
                    Some(tree_sitter_info) => {
                        tree_sitter_info.tree = tree_sitter_info.parser.parse(&self.contents.to_string(),None).unwrap();//TODO: replace this with an incremental parse
                    },
                    None => {}
                }
            },
            "change_tab" => {
                if let Some(tab) = command_args.next() {
                    if let Ok(tab) = tab.parse::<usize>() {
                        self.editor_sender.send(EditorMessage::NthWindow(tab)).expect("Failed to send message");
                    }
                    else {
                        match tab {
                            "prev" => {
                                self.editor_sender.send(EditorMessage::PrevWindow).expect("Failed to send message");
                            },
                            "next" => {
                                self.editor_sender.send(EditorMessage::NextWindow).expect("Failed to send message");
                            },
                            _ => {}
                        }
                    }
                }
            },
            "open_tab" => {
                //self.window_sender.send(WindowMessage::OpenNewTab).expect("Failed to send message");
                self.editor_sender.send(EditorMessage::NewWindow(None)).expect("Failed to send message");
            },
            "open_tab_with_pane" => {
                self.window_sender.send(WindowMessage::OpenNewTabWithPane).expect("Failed to send message");
            },
            "info" => {
                //self.open_info(container);
            },
            "completion" => {

                let mut cont = true;
                
                match self.lsp_info.take() {
                    None => {},
                    Some(mut lsp_info) => {

                        let LspInfo {lang, lsp_client: (sender, _),  ..} = &mut lsp_info;

                        let uri = self.generate_uri();

                        let position = self.cursor.borrow().get_cursor();

                        sender.send(LspControllerMessage::Request(
                             lang.clone().into(),
                            LspRequest::RequestCompletion(uri.into(), position, "invoked".into())
                        )).expect("Failed to send message");

                        cont = false;

                        self.lsp_info = Some(lsp_info);
                    }
                }

                if !cont {
                    while self.lsp_info.as_ref().unwrap().lsp_completion.is_none() {
                        self.read_lsp_messages();
                    }
                }
                
                match self.lsp_info.take() {
                    None => {},
                    Some(mut lsp_info) => {

                        let LspInfo { lsp_completion, ..} = &mut lsp_info;
                        

                        let completion_list = match lsp_completion.take() {
                            None => return,
                            Some(list) => list,
                        };

                        let buttons = completion_list.generate_buttons(70);


                        
                        let (send, recv) = std::sync::mpsc::channel();
                        let (send2, recv2) = std::sync::mpsc::channel();

                        self.popup_channels = vec![(send2, recv)];

                        let prompt = Vec::new();

                        let size = (70, buttons.button_len().expect("Buttons were not buttons") + 2);

                        
                        let pane = PopUpPane::new_dropdown(
                            self.settings.clone(),
                            prompt,
                            self.window_sender.clone(),
                            send,
                            recv2,
                            buttons,
                            false,
                        );

                        let pane = Rc::new(RefCell::new(pane));

                        let pos = self.cursor.borrow().get_real_cursor();
                        

                        let max_size = container.get_size();

                        let mut container = PaneContainer::new(pane, self.settings.clone());


                        container.set_position(pos);
                        container.set_size(size);



                        self.window_sender.send(WindowMessage::CreatePopup(container, true)).expect("Failed to send message");
                        self.waiting = Waiting::Completion;

                        self.contents.add_new_rope();

                        *lsp_completion = Some(completion_list);

                        self.lsp_info = Some(lsp_info);
                    },
                }
                
            },
            "insert" => {
                if let Some(index) = command_args.next() {
                    //eprintln!("Inserting {}", index);

                    if let Some(index) = index.parse::<usize>().ok() {
                        match self.lsp_info.take() {
                            None => {},
                            Some(mut lsp_info) => {
                                let LspInfo { lsp_completion, ..} = &mut lsp_info;
                                
                                match lsp_completion {
                                    None => {},
                                    Some(ref mut list) => {
                                        //eprintln!("Inserting {}", index);
                                        if let Some(completion) = list.get_completion(index) {
                                            //eprintln!("Inserting {}", index);
                                            if let Some(text_edit) = completion.get_edit_text() {
                                                //eprintln!("Inserting {}", index);
                                                match text_edit {
                                                    TextEditType::TextEdit(text_edit) => {
                                                        let (pos, _) = text_edit.get_range();

                                                        self.insert_str_at(pos, &text_edit.newText);
                                                        //eprintln!("Inserting {} at {:?}", &text_edit.newText, pos);
                                                    },
                                                    TextEditType::InsertReplaceEdit(text_edit) => {
                                                        unimplemented!();
                                                    },
                                                }
                                            }
                                        }
                                    },
                                }
                            },
                        }

                    }
                }

                match &mut self.lsp_info {
                    None => {},
                    Some(LspInfo {
                        lsp_completion,
                        ..
                    }) => {
                        lsp_completion.take();
                    },
                }
                
            },
            "goto_declaration" | "goto_definition" |
            "goto_type_definition" | "goto_implementation" => {

                let mut cont = true;
                match self.lsp_info.take() {
                    None => {},
                    Some(mut lsp_info) => {

                        let LspInfo {lang, lsp_client: (sender, _),  ..} = &mut lsp_info;
                        let uri = self.generate_uri();

                        let position = self.cursor.borrow().get_cursor();

                        let request = match command {
                            "goto_declaration" => LspRequest::GotoDeclaration(uri.clone().into(), position),
                            "goto_definition" => LspRequest::GotoDefinition(uri.clone().into(), position),
                            "goto_type_definition" => LspRequest::GotoTypeDefinition(uri.clone().into(), position),
                            "goto_implementation" => LspRequest::GotoImplementation(uri.clone().into(), position),
                            _ => unreachable!(),
                        };


                        sender.send(LspControllerMessage::Request(
                             lang.clone().into(),
                            request
                        )).expect("Failed to send message");
                        cont = false;

                        self.lsp_info = Some(lsp_info);
                    }
                }

                if !cont {
                    while self.lsp_info.as_ref().unwrap().lsp_location.is_none() {
                        self.read_lsp_messages();
                    }
                }

                match self.lsp_info.take() {
                    None => {},
                    Some(mut lsp_info) => {

                        let LspInfo { lang, lsp_client: (sender, _), lsp_location, ..} = &mut lsp_info;

                        let location = lsp_location.take().expect("LSP location was none");

                        let uri = self.generate_uri();


                        match location {
                            LocationResponse::Location(location) => {
                                //eprintln!("Got location {:?}", location);
                                
                                if location.uri.as_str() == uri.as_str() {
                                    //eprintln!("Jumping to {:?}", location.range);
                                    self.jump_table.add(*self.cursor.borrow());

                                    let ((x, y), _) = location.range.get_positions();

                                    let mut cursor = self.cursor.borrow_mut();

                                    cursor.set_cursor(CursorMove::Amount(x), CursorMove::Amount(y), self, (0,0));
                                }
                                else {

                                    let file_name = Self::get_file_path(&location.uri);
                                    let (pos, _) = location.range.get_positions();

                                    let message = WindowMessage::OpenFile(file_name, Some(pos));

                                    self.window_sender.send(message).expect("Failed to send message");

                                    self.contents.add_new_rope();
                                }
                            },
                            LocationResponse::Locations(locations) => {

                                if locations.len() == 1 {
                                    
                                    if locations[0].uri.as_str() == uri.as_str() {
                                        //eprintln!("Jumping to {:?}", locations[0].range);
                                        self.jump_table.add(*self.cursor.borrow());

                                        let ((x, y), _) = locations[0].range.get_positions();

                                        let mut cursor = self.cursor.borrow_mut();

                                        cursor.set_cursor(CursorMove::Amount(x), CursorMove::Amount(y), self, (0,0));
                                    }
                                    else {

                                        let file_name = Self::get_file_path(&locations[0].uri);
                                        let (pos, _) = locations[0].range.get_positions();

                                        let message = WindowMessage::OpenFile(file_name, Some(pos));

                                        self.window_sender.send(message).expect("Failed to send message");
                                        
                                        self.contents.add_new_rope();
                                    }
                                }
                                else {
                                    let (send, recv) = std::sync::mpsc::channel();
                                    let (send2, recv2) = std::sync::mpsc::channel();

                                    self.popup_channels = vec![(send2, recv)];

                                    let mut buttons = Vec::new();
                                    
                                    for location in locations.iter() {
                                        let pathbuf = PathBuf::from(location.uri.clone());
                                        let file_name = pathbuf.file_name().expect("Failed to get file name").to_str().expect("Failed to convert to str").to_string();
                                        let location = location.clone();
                                        
                                        let function: Box<dyn Fn(&dyn Promptable) -> String> = Box::new(move |_| {
                                            format!("{} {},{}", Self::get_file_path(&location.uri), location.range.start.character, location.range.start.line)
                                        });
                                        
                                        buttons.push((file_name, function));
                                        

                                    }

                                    let buttons = PromptType::Button(buttons, 0);
                                    let prompt = vec!["Locations".to_string()];

                                    let pane = PopUpPane::new_dropdown(
                                        self.settings.clone(),
                                        prompt,
                                        self.window_sender.clone(),
                                        send,
                                        recv2,
                                        buttons,
                                        true
                                    );

                                    let pane = Rc::new(RefCell::new(pane));

                                    let (_, (x2, y2)) = container.get_corners();
                                    let (x, y) = container.get_size();

                                    let (x, y) = (x / 2, y / 2);

                                    let pos = (x2 - 20 - x, y2 - (locations.len() + 3) - y);


                                    let max_size = container.get_size();
                                    
                                    let mut container = PaneContainer::new(pane, self.settings.clone());


                                    container.set_position(pos);
                                    container.set_size((20, locations.len() + 3));



                                    self.window_sender.send(WindowMessage::CreatePopup(container, true)).expect("Failed to send message");
                                    self.waiting = Waiting::Goto;

                                    self.contents.add_new_rope();
                                    
                                }

                            },
                            LocationResponse::LocationLink(location_link) => {
                                eprintln!("Got location link {:?}", location_link);

                            },
                            LocationResponse::Null => {},
                            
                            
                        }
                        
                        self.lsp_info = Some(lsp_info);
                    }
                }


            },
            "goto" => {
                if let Some(path) = command_args.next() {
                    let pos = if let Some(pos) = command_args.next() {
                        let split = pos.split(",").collect::<Vec<_>>();
                        let x = split[0].parse::<usize>().expect("Failed to parse x");
                        let y = split[1].parse::<usize>().expect("Failed to parse y");
                        Some((x, y))
                    }
                    else {
                        None
                    };

                    let message = WindowMessage::OpenFile(path.to_string(), pos);

                    self.window_sender.send(message).expect("Failed to send message");
                    
                }
            },
            /*"paste" => {
                if let Some(arg) = command_args.next() {
                    if let Ok(number) = arg.parse::<usize>() {
                        let message = WindowMessage::Paste(RegisterType::Number(number));

                        self.window_sender.send(message).expect("Failed to send message");
                    } else {
                        let message = WindowMessage::Paste(RegisterType::Name(arg.to_string()));

                        self.window_sender.send(message).expect("Failed to send message");
                    }
                } else {
                    let message = WindowMessage::Paste(RegisterType::None);

                    self.window_sender.send(message).expect("Failed to send message");
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

                            let message = WindowMessage::Copy(reg, row);

                            self.window_sender.send(message).expect("Failed to send message");
                                    
                        },
                        _ => {},

                    }
                        

                }

            },*/
                

            _ => {}
        }

    }

    fn resize(&mut self, size: (usize, usize)) {
        self.cursor.borrow_mut().resize(size);
    }

    fn set_location(&mut self, location: (usize, usize)) {
        let cursor = self.cursor.clone();
        cursor.borrow_mut().set_cursor(CursorMove::Amount(location.0),
                                       CursorMove::Amount(location.1),
                                       self, (0,0));
    }

    fn get_settings(&self) -> Rc<RefCell<Settings>> {
        self.settings.clone()
    }

    fn change_mode(&mut self, mode: &str) {
        let mode = self.modes.get(&mode.to_owned()).unwrap();
        self.mode = mode.clone();
    }

    
    fn backup(&mut self) {
        self.backup_buffer();

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

        let file_type = filename.extension().and_then(|s| s.to_str()).unwrap_or("txt").to_string();

        let file = File::open(filename.clone())?;
        let mut file = BufReader::new(file);
        let mut contents = String::new();
        let _amount_read = file.read_to_string(&mut contents)?;
        self.contents = Buffer::from(contents);
        self.file_name = Some(filename);

        match file_type.as_str() {
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

                let lsp_client = (self.lsp_sender.clone(), lsp_client);

                let language = tree_sitter_rust::language();
                let mut parser = Parser::new();
                parser.set_language(language).unwrap();

                let tree = parser.parse(self.contents.to_string(), None).unwrap();

                let tree_sitter = TreeSitterInfo::new(parser, tree, "rust".to_string());

                self.tree_sitter_info = Some(tree_sitter);

                let lsp_client = LspInfo::new("rust", lsp_client);

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

                let lsp_client = (self.lsp_sender.clone(), lsp_client);

                let lsp_client = LspInfo::new("c", lsp_client);

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

                let lsp_client = (self.lsp_sender.clone(), lsp_client);

                let lsp_client = LspInfo::new("cpp", lsp_client);

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

                let lsp_client = (self.lsp_sender.clone(), lsp_client);

                let lsp_client = LspInfo::new("python", lsp_client);

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

                let lsp_client = (self.lsp_sender.clone(), lsp_client);

                let lsp_client = LspInfo::new("swift", lsp_client);

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

                let lsp_client = (self.lsp_sender.clone(), lsp_client);

                let lsp_client = LspInfo::new("go", lsp_client);

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

                let lsp_client = (self.lsp_sender.clone(), lsp_client);

                let lsp_client = LspInfo::new("bash", lsp_client);

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

        match &mut self.tree_sitter_info {
            None => {},
            Some(TreeSitterInfo { parser, tree, language }) => {
                let edit = InputEdit {
                    start_byte,
                    old_end_byte: start_byte,
                    new_end_byte,
                    start_position: Point::new(y, x),
                    old_end_position: Point::new(y, x + insert_char_count),
                    new_end_position: Point::new(y, new_char_len),
                };

                tree.edit(&edit);
                *tree = parser.parse(self.contents.to_string(), Some(&tree)).unwrap();
            }
        }

        match self.lsp_info.take() {
            None => {},
            Some(mut lsp_info) => {

                let LspInfo { lang, lsp_client: (sender, _), file_version, .. } = &mut lsp_info;
                *file_version += 1;
                let message = LspControllerMessage::Notification(
                    lang.clone().into(),
                    LspNotification::ChangeText(
                        self.generate_uri().into(),
                        *file_version,
                        self.contents.to_string().into(),
                    )
                );

                sender.send(message).expect("Failed to send message");
                self.lsp_info = Some(lsp_info);
            }
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

        match &mut self.tree_sitter_info {
            None => {},
            Some(TreeSitterInfo { parser, tree, language }) => {
                let edit = InputEdit {
                    start_byte,
                    old_end_byte,
                    new_end_byte,
                    start_position: Point::new(y, x),
                    old_end_position: Point::new(y, x - 1),
                    new_end_position: Point::new(y, x),
                };

                tree.edit(&edit);
                *tree = parser.parse(self.contents.to_string(), Some(&tree)).unwrap();
            }
        }

        match self.lsp_info.take() {
            None => {},
            Some(mut lsp_info) => {

                let LspInfo { lang, lsp_client: (sender, _), file_version, .. } = &mut lsp_info;
                *file_version += 1;
                let message = LspControllerMessage::Notification(
                    lang.clone().into(),
                    LspNotification::ChangeText(
                        self.generate_uri().into(),
                        *file_version,
                        self.contents.to_string().into(),
                    )
                );

                sender.send(message).expect("Failed to send message");
                self.lsp_info = Some(lsp_info);
            }
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

        match &mut self.tree_sitter_info {
            None => {},
            Some(TreeSitterInfo { parser, tree, language }) => {
                let edit = InputEdit {
                    start_byte,
                    old_end_byte,
                    new_end_byte,
                    start_position: Point::new(y, x),
                    old_end_position: Point::new(y, x.saturating_sub(1)),
                    new_end_position: Point::new(y, x),
                };

                tree.edit(&edit);
                *tree = parser.parse(self.contents.to_string(), Some(&tree)).unwrap();
            }
        }

        match self.lsp_info.take() {
            None => {},
            Some(mut lsp_info) => {

                let LspInfo { lang, lsp_client: (sender, _), file_version, .. } = &mut lsp_info;
                *file_version += 1;
                let message = LspControllerMessage::Notification(
                    lang.clone().into(),
                    LspNotification::ChangeText(
                        self.generate_uri().into(),
                        *file_version,
                        self.contents.to_string().into(),
                    )
                );

                sender.send(message).expect("Failed to send message");
                self.lsp_info = Some(lsp_info);
            }
        }
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

    fn get_physical_cursor(&self) -> Rc<RefCell<Cursor>> {
        self.cursor.clone()
    }
}



