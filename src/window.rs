use core::fmt;
use std::cell::RefCell;
use std::cmp;
use std::collections::{HashSet, HashMap};
use std::fmt::{Debug, Formatter};
use std::ops::Index;
use std::path::PathBuf;
use std::rc::Rc;
use std::io;
use std::io::Write;
use std::sync::mpsc::{Sender, Receiver, self, TryRecvError};
use std::time::Duration;

use crossterm::event::{KeyEvent, self, Event};
use crossterm::style::{Stylize, StyledContent};
use crossterm::{terminal::{self, ClearType}, execute, cursor, queue};
use uuid::Uuid;

use crate::Mailbox;
use crate::editor::{EditorMessage, RegisterType};
use crate::lsp::LspControllerMessage;
use crate::pane::treesitter::TreesitterPane;
use crate::settings::ColorScheme;
use crate::{apply_colors, settings::Settings};
use crate::pane::{Pane, PaneContainer};
use crate::pane::text::PlainTextPane;
use crate::treesitter::tree_sitter_scheme;


pub enum WindowMessage {
    HorizontalSplit,
    VerticalSplit,
    ForceQuitAll,
    PaneUp,
    PaneDown,
    PaneLeft,
    PaneRight,
    OpenFile(String, Option<(usize, usize)>),
    /// go down a layer
    ClosePane(bool, Option<Uuid>),
    CreatePopup(PaneContainer, bool),
    OpenNewTab,
    OpenNewTabWithPane,
    NextTab,
    PreviousTab,
    NthTab(usize),
    PasteResponse(Option<Box<str>>),
    Paste(RegisterType),
    Copy(RegisterType, String),
}

pub struct WindowMailbox {
    /// The receiver for messages to the editor
    local_receiver: Receiver<WindowMessage>,
    /// The sender for messages to the editor
    /// This isn't wrapped in an Rc because it is easy to share
    far_sender: Sender<WindowMessage>,
    /// The receiver for messages not to the editor
    /// This is wrapped in an Rc so that it can be shared with other parts of the editor
    far_receiver: Rc<Receiver<WindowMessage>>,
    /// The sender for messages not to the editor
    local_sender: Sender<WindowMessage>,
}

impl WindowMailbox {
    pub fn new() -> Self {
        let (far_sender , far_receiver) = std::sync::mpsc::channel();
        let (local_sender, local_receiver) = std::sync::mpsc::channel();

        Self {
            local_receiver,
            far_sender,
            far_receiver: Rc::new(far_receiver),
            local_sender,
        }
    }

    pub fn get_far_receiver(&self) -> Rc<Receiver<WindowMessage>> {
        self.far_receiver.clone()
    }

    pub fn get_far_sender(&self) -> Sender<WindowMessage> {
        self.far_sender.clone()
    }
    
}


impl Mailbox<WindowMessage> for WindowMailbox {
    fn send(&self, message: WindowMessage) -> Result<(), std::sync::mpsc::SendError<WindowMessage>> {
        self.local_sender.send(message)
    }

    fn recv(&self) -> Result<WindowMessage, std::sync::mpsc::RecvError> {
        self.local_receiver.recv()
    }

    fn try_recv(&self) -> Result<WindowMessage, std::sync::mpsc::TryRecvError> {
        self.local_receiver.try_recv()
    }
}



pub struct Window{
    size: (usize, usize),
    contents: WindowContents,
    active_panes: Vec<usize>,
    active_layer: usize,
    panes: Vec<Vec<PaneContainer>>,
    id_to_pane: HashMap<Uuid, (usize, usize)>,
    buffers: Vec<TextBuffer>,
    compositor: Compositor,
    known_file_types: HashSet<String>,
    settings: Rc<RefCell<Settings>>,
    duration: Duration,
    channels: (Sender<WindowMessage>, Receiver<WindowMessage>),
    editor_sender: Sender<EditorMessage>,
    /// For when we want to avoid waiting for an event to be processed
    skip: bool,
    lsp_sender: Sender<LspControllerMessage>,
    lsp_listener: Rc<Receiver<LspControllerMessage>>,
    
}

impl Window {
    pub fn new(editor_sender: Sender<EditorMessage>,
               editor_listener: Rc<Receiver<EditorMessage>>,
               lsp_sender: Sender<LspControllerMessage>,
               lsp_listener: Rc<Receiver<LspControllerMessage>>,
               settings: Rc<RefCell<Settings>>) -> Self {
    
        //let settings = Settings::default();
                                                             
        let duration = Duration::from_millis(settings.borrow().editor_settings.key_timeout);

        //let settings = Rc::new(RefCell::new(settings));
        

        let channels = mpsc::channel();
        
        /*let win_size = terminal::size()
            .map(|(w, h)| (w as usize, h as usize - 1))// -1 for trailing newline and -1 for command bar
        .unwrap();*/

        let win_size = (settings.borrow().cols, settings.borrow().rows -1);
        let pane: Rc<RefCell<dyn Pane>> = Rc::new(RefCell::new(PlainTextPane::new(settings.clone(), channels.0.clone())));

        pane.borrow_mut().set_cursor_size(win_size);
        
        let panes = vec![vec![PaneContainer::new(win_size, win_size, pane.clone(), settings.clone())]];

        let mut id_to_pane = HashMap::new();
        id_to_pane.insert(panes[0][0].get_uuid(), (0, 0));

        let mut known_file_types = HashSet::new();
        known_file_types.insert("txt".to_string());
        let buffers = vec![TextBuffer::new(); 1];
        
        Self {
            size: win_size,
            contents: WindowContents::new(),
            active_panes: vec![0],
            active_layer: 0,
            panes,
            id_to_pane,
            buffers,
            compositor: Compositor::new((win_size.0, win_size.1)),
            known_file_types,
            duration,
            settings,
            channels,
            editor_sender,
            skip: false,
            lsp_listener,
            lsp_sender,
        }
    }

    pub fn get_sender(&self) -> Sender<WindowMessage> {
        self.channels.0.clone()
    }

    fn create_popup(&mut self, pane: PaneContainer, make_active: bool) {

        /*let mut offset = self.active_layer;
        while self.panes.len() <= offset {
            if offset == self.panes.len() {
                eprintln!("Creating new layer");
                self.panes.push(vec![pane]);
                self.buffers.push(TextBuffer::new());
                break;
            }
            if self.panes[offset].len() == 0 {
                eprintln!("Adding to existing layer");
                self.panes[offset].push(pane);
                break;
            }
            offset += 1;
        }
        if make_active {
            eprintln!("Making new pane active");
            self.active_layer = offset;
            self.active_panes[self.active_layer] = self.panes[self.active_layer].len() - 1;

        }*/

        
        if self.panes.len() - 1 == self.active_layer {
            //eprintln!("Creating new layer");
            self.panes.push(vec![pane]);
        }
        else {
            //eprintln!("Adding to existing layer");
            self.panes[self.active_layer + 1].push(pane);
        }
        if self.buffers.len() - 1 == self.active_layer {
            //eprintln!("Creating new buffer");
            self.buffers.push(TextBuffer::new());
        }
        if self.active_panes.len() - 1 == self.active_layer {
            //eprintln!("Creating new active pane");
            self.active_panes.push(self.panes[self.active_layer].len() - 1);
        }
        else {
            //eprintln!("Adding to existing active pane");
            self.active_panes[self.active_layer + 1] = self.panes[self.active_layer].len() - 1;
        }
        if make_active {
            //eprintln!("Making new pane active");
            self.active_layer = self.panes.len() - 1;

        }
    }

    fn file_opener(&mut self, filename: PathBuf) -> io::Result<Rc<RefCell<dyn Pane>>> {
        //eprintln!("Opening file: {:?}", filename);
        let file_type = filename.extension().and_then(|s| s.to_str()).unwrap_or("txt").to_string();

        let pane: Rc<RefCell<dyn Pane>> = match file_type.as_str() {
            "scm" => {
                let language = unsafe { tree_sitter_scheme() };
                let mut pane = TreesitterPane::new(self.settings.clone(), self.channels.0.clone(), language, "scheme", None);
                pane.open_file(&filename)?;
                pane.backup_buffer();
                Rc::new(RefCell::new(pane))
            },
            "rs" => {

                self.lsp_sender.send(LspControllerMessage::CreateClient("rust".to_string().into())).unwrap();

                let lsp_client;

                loop {
                        
                    match self.lsp_listener.try_recv() {
                        Ok(LspControllerMessage::ClientCreated(language_rcv)) => {
                            lsp_client = language_rcv;
                            break;
                        },
                        Ok(_) => {
                            continue;
                        },
                        Err(TryRecvError::Empty) => {
                            continue;
                        },
                        Err(TryRecvError::Disconnected) => {
                            unreachable!();
                        },
                    }
                }

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));
                
                let language = tree_sitter_rust::language();
                let mut pane = TreesitterPane::new(self.settings.clone(), self.channels.0.clone(), language,"rust", lsp_client);
                pane.open_file(&filename)?;
                pane.backup_buffer();
                Rc::new(RefCell::new(pane))
            }
            //todo: move h to C++ since there is no easy way of knowing which lang it is
            "c" | "h" => {

                self.lsp_sender.send(LspControllerMessage::CreateClient("c".to_string().into())).unwrap();

                let lsp_client;
                
                loop {

                    match self.lsp_listener.try_recv() {
                        Ok(LspControllerMessage::ClientCreated(language_rcv)) => {
                            lsp_client = language_rcv;
                            break;
                        },
                        Ok(_) => {
                            continue;
                        },
                        Err(TryRecvError::Empty) => {
                            continue;
                        },
                        Err(TryRecvError::Disconnected) => {
                            unreachable!();
                        },
                    }
                }

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));
                
                let language = tree_sitter_c::language();
                let mut pane = TreesitterPane::new(self.settings.clone(), self.channels.0.clone(), language,"c", lsp_client);
                pane.open_file(&filename)?;
                pane.backup_buffer();
                Rc::new(RefCell::new(pane))
            }
            "cpp" | "hpp" /*| "h"*/ => {

                self.lsp_sender.send(LspControllerMessage::CreateClient("cpp".to_string().into())).unwrap();

                let lsp_client = match self.lsp_listener.recv().unwrap() {
                    LspControllerMessage::ClientCreated(language_rcv) => {
                        language_rcv
                    },
                    _ => unreachable!(),
                };

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));
                
                let language = tree_sitter_cpp::language();
                let mut pane = TreesitterPane::new(self.settings.clone(), self.channels.0.clone(), language,"cpp", lsp_client);
                pane.open_file(&filename)?;
                pane.backup_buffer();
                Rc::new(RefCell::new(pane))
            }
            "py" => {

                self.lsp_sender.send(LspControllerMessage::CreateClient("python".to_string().into())).unwrap();

                let lsp_client = match self.lsp_listener.recv().unwrap() {
                    LspControllerMessage::ClientCreated(language_rcv) => {
                        language_rcv
                    },
                    _ => unreachable!(),
                };

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));
                
                let language = tree_sitter_python::language();
                let mut pane = TreesitterPane::new(self.settings.clone(), self.channels.0.clone(), language,"python", lsp_client);
                pane.open_file(&filename)?;
                pane.backup_buffer();
                Rc::new(RefCell::new(pane))
            }
            "lsp" => {
                let language = tree_sitter_commonlisp::language();
                let mut pane = TreesitterPane::new(self.settings.clone(), self.channels.0.clone(), language,"commonlisp", None);
                pane.open_file(&filename)?;
                pane.backup_buffer();
                Rc::new(RefCell::new(pane))
            }
            "swift" => {

                self.lsp_sender.send(LspControllerMessage::CreateClient("swift".to_string().into())).unwrap();

                let lsp_client = match self.lsp_listener.recv().unwrap() {
                    LspControllerMessage::ClientCreated(language_rcv) => {
                        language_rcv
                    },
                    _ => unreachable!(),
                };

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));
                
                let language = tree_sitter_swift::language();
                let mut pane = TreesitterPane::new(self.settings.clone(), self.channels.0.clone(), language,"swift", lsp_client);
                pane.open_file(&filename)?;
                pane.backup_buffer();
                Rc::new(RefCell::new(pane))
            }
            "go" => {

                self.lsp_sender.send(LspControllerMessage::CreateClient("go".to_string().into())).unwrap();

                let lsp_client = match self.lsp_listener.recv().unwrap() {
                    LspControllerMessage::ClientCreated(language_rcv) => {
                        language_rcv
                    },
                    _ => unreachable!(),
                };

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));
                
                let language = tree_sitter_go::language();
                let mut pane = TreesitterPane::new(self.settings.clone(), self.channels.0.clone(), language,"go", lsp_client);
                pane.open_file(&filename)?;
                pane.backup_buffer();
                Rc::new(RefCell::new(pane))
            }
            "sh" => {

                self.lsp_sender.send(LspControllerMessage::CreateClient("bash".to_string().into())).unwrap();

                let lsp_client = match self.lsp_listener.recv().unwrap() {
                    LspControllerMessage::ClientCreated(language_rcv) => {
                        language_rcv
                    },
                    _ => unreachable!(),
                };

                let lsp_client = Some((self.lsp_sender.clone(), lsp_client));
                
                let language = tree_sitter_bash::language();
                let mut pane = TreesitterPane::new(self.settings.clone(), self.channels.0.clone(), language,"bash", lsp_client);
                pane.open_file(&filename)?;
                pane.backup_buffer();
                Rc::new(RefCell::new(pane))
            }
            "js" => {
                let language = tree_sitter_javascript::language();
                let mut pane = TreesitterPane::new(self.settings.clone(), self.channels.0.clone(), language,"javascript", None);
                pane.open_file(&filename)?;
                pane.backup_buffer();
                Rc::new(RefCell::new(pane))
            }
            "cs" => {
                let language = tree_sitter_c_sharp::language();
                let mut pane = TreesitterPane::new(self.settings.clone(), self.channels.0.clone(), language,"csharp", None);
                pane.open_file(&filename)?;
                pane.backup_buffer();
                Rc::new(RefCell::new(pane))
            }
            "txt" | _ => {
                let mut pane = PlainTextPane::new(self.settings.clone(), self.channels.0.clone());
                pane.open_file(&filename)?;
                pane.backup_buffer();
                Rc::new(RefCell::new(pane))
            }
        };
        Ok(pane)
    }

    fn open_file(&mut self, filename: PathBuf) -> io::Result<usize> {

        let pane: Rc<RefCell<dyn Pane>> = self.file_opener(filename)?;
        self.panes[self.active_layer].push(PaneContainer::new((0,0), (0, 0), pane.clone(), self.settings.clone()));
        Ok(self.panes[self.active_layer].len() - 1)
    }

    fn switch_pane(&mut self, filename: String, pos: Option<(usize, usize)>) -> io::Result<()> {
        let filename = PathBuf::from(filename);
        //eprintln!("switching to pane: {:?}", filename);
        let mut pane_index = None;
        for (i, pane) in self.panes[self.active_layer].iter().enumerate() {
            if pane.get_pane().borrow().get_filename() == &Some(filename.clone()) {//todo: remove the clone
                pane_index = Some(i);
                break;
            }
        }

        let new_active_pane_index = if let Some(pane_index) = pane_index {
            pane_index
        }
        else {
            self.open_file(filename)?
        };

        let active_pane = self.panes[self.active_layer][self.active_panes[self.active_layer]].get_pane().clone();
        let new_active_pane = self.panes[self.active_layer][new_active_pane_index].get_pane().clone();

        active_pane.borrow_mut().reset();

        self.panes[self.active_layer][self.active_panes[self.active_layer]].change_pane(new_active_pane);
        self.panes[self.active_layer][new_active_pane_index].change_pane(active_pane);

        self.panes[self.active_layer][self.active_panes[self.active_layer]].get_pane().borrow_mut().set_cursor_size(self.panes[self.active_layer][self.active_panes[self.active_layer]].get_size());

        if let Some((x, y)) = pos {
            let pane = self.panes[self.active_layer][self.active_panes[self.active_layer]].get_pane().clone();
            
            pane.borrow()
                .get_cursor()
                .borrow_mut()
                .set_cursor(crate::cursor::CursorMove::Amount(x),
                            crate::cursor::CursorMove::Amount(y),
                            &*pane.borrow(),
                            (0, 0));
        }

        self.panes[self.active_layer][self.active_panes[self.active_layer]].scroll_cursor();
        self.force_refresh_screen()?;
        
        Ok(())
    }

    pub fn insert_pane(&mut self, index: usize, pane: PaneContainer) {
        let parent_pane = index - 1;
        self.panes[self.active_layer].insert(index, pane);
    }

    pub fn replace_pane(&mut self, index: usize, pane: Rc<RefCell<dyn Pane>>) {
        let cursor = self.panes[self.active_layer][index].get_cursor();
        {
            let mut pane = pane.borrow_mut();
            pane.set_sender(self.channels.0.clone());
        }
        self.panes[self.active_layer][index].change_pane(pane);
        let new_cursor = self.panes[self.active_layer][index].get_cursor();
        let mut new_cursor = new_cursor.borrow_mut();
        new_cursor.prepare_jump(&*cursor.borrow());
        new_cursor.jumped = false;
    }

    fn remove_panes(&mut self) {
        let mut panes_to_remove = Vec::new();
        for (i, layer) in self.panes.iter().enumerate() {
            for (j, pane) in layer.iter().enumerate() {
                if pane.can_close() {
                    panes_to_remove.push((i, j));
                }
            }
        }
        
        for (i, j) in panes_to_remove.iter().rev() {
            
            loop {
                if *j + 1 < self.panes[*i].len() {
                    let corners = self.panes[*i][*j].get_corners();
                    if self.panes[*i][*j + 1].combine(corners) {
                        break;
                    }
                }
                if *j != 0 {
                    let corners = self.panes[*i][*j].get_corners();
                    if self.panes[*i][*j - 1].combine(corners) {
                        break;
                    }
                }
                break;
            }

            
            self.panes[*i].remove(*j);
        }

        for (i, layer) in self.panes.iter().enumerate() {
            if layer.len() == 0 {
                self.active_panes[i] = 0;
            }
            else {
                self.active_panes[i] = cmp::min(self.active_panes[i], layer.len() - 1);
            }
        }

        self.id_to_pane = HashMap::new();

        for (i, layer) in self.panes.iter().enumerate() {
            for (j, pane) in layer.iter().enumerate() {
                self.id_to_pane.insert(pane.get_uuid(), (i, j));
            }
        }
           
    }



    fn horizontal_split(&mut self) {
        //eprintln!("split panes: {:?}", self.panes.len());
        let active_pane_size = self.panes[self.active_layer][self.active_panes[self.active_layer]].get_size();
        let new_pane_size = if active_pane_size.0 % 2 == 0 {
            (active_pane_size.0, active_pane_size.1 / 2)
        }
        else {
            (active_pane_size.0, active_pane_size.1 / 2 -1)
        };
        let old_pane_size = if active_pane_size.1 % 2 == 0 {
            (new_pane_size.0, new_pane_size.1)
        }
        else {
            (new_pane_size.0, new_pane_size.1 + 1)
        };

            
        self.panes[self.active_layer][self.active_panes[self.active_layer]].set_size(old_pane_size);


        let new_pane_index = self.active_panes[self.active_layer] + 1;
        let mut new_pane = self.panes[self.active_layer][self.active_panes[self.active_layer]].clone();

        let ((x,_), (_, y)) = self.panes[self.active_layer][self.active_panes[self.active_layer]].get_corners();
        
        let new_pane_position = (x, y + 1);

        new_pane.set_position(new_pane_position);
        self.panes[self.active_layer].insert(new_pane_index, new_pane);


        // This is for testing purposes, we need to make sure that we can actually access the new pane
        self.active_panes[self.active_layer] = new_pane_index;

        self.id_to_pane.insert(self.panes[self.active_layer][new_pane_index].get_uuid(), (self.active_layer, new_pane_index));

        //eprintln!("split panes: {:?}", self.panes.len());
    }

    fn vertical_split(&mut self) {
        let active_pane_size = self.panes[self.active_layer][self.active_panes[self.active_layer]].get_size();
        let new_pane_size = if active_pane_size.0 % 2 == 0 {
            (active_pane_size.0 / 2, active_pane_size.1)
        }
        else {
            (active_pane_size.0 / 2 - 1, active_pane_size.1)
        };
        let old_pane_size = if active_pane_size.0 % 2 == 0 {
            new_pane_size
        }
        else {
            (new_pane_size.0 + 1, new_pane_size.1)
        };
        
        self.panes[self.active_layer][self.active_panes[self.active_layer]].set_size(old_pane_size);


        let new_pane_index = self.active_panes[self.active_layer] + 1;
        let mut new_pane = self.panes[self.active_layer][self.active_panes[self.active_layer]].clone();

        let ((_,y), (x, _)) = self.panes[self.active_layer][self.active_panes[self.active_layer]].get_corners();
        let new_pane_position = (x + 1, y);

        new_pane.set_position(new_pane_position);
        self.panes[self.active_layer].insert(new_pane_index, new_pane);

        //eprintln!("old corners {:?}", self.panes[self.active_layer][self.active_panes[self.active_layer]].get_corners());
        
        self.active_panes[self.active_layer] = new_pane_index;

        self.id_to_pane.insert(self.panes[self.active_layer][new_pane_index].get_uuid(), (self.active_layer, new_pane_index));

        //eprintln!("new corners {:?}", self.panes[self.active_layer][self.active_panes[self.active_layer]].get_corners());
    }


    fn pane_up(&mut self) {
        let ((x1, y1), (x2, _)) = self.panes[self.active_layer][self.active_panes[self.active_layer]].get_corners();

        let pane_top = y1.saturating_sub(1);

        let pane_middle = (x1 + x2) / 2;

        let mut pane_index = None;
        // This loop tries to find the pane that is above the current pane
        for (i, pane) in self.panes[self.active_layer].iter().enumerate() {
            let ((x1, _), (x2, y2)) = pane.get_corners();
            if y2 == pane_top && x1 <= pane_middle && pane_middle <= x2 {
                pane_index = Some(i);
                break;
            }
        }

        match pane_index {
            Some(index) => {
                self.active_panes[self.active_layer] = index;
            }
            None => {}
        }
    }

    fn pane_down(&mut self) {
        let ((x1, _), (x2, y2)) = self.panes[self.active_layer][self.active_panes[self.active_layer]].get_corners();

        // We add 1 to make sure that we aren't on the current pane
        let pane_bottom = y2 + 1;
        let pane_middle = (x1 + x2) / 2;

        let mut pane_index = None;
        // This loop tries to find the pane that is below the current pane
        for (i, pane) in self.panes[self.active_layer].iter().enumerate() {
            let ((x1, y1), (x2, _)) = pane.get_corners();
            if y1 == pane_bottom && x1 <= pane_middle && pane_middle <= x2 {
                pane_index = Some(i);
                break;
            }
        }

        match pane_index {
            Some(index) => {
                self.active_panes[self.active_layer] = index;
            }
            None => {}
        }
    }

    fn pane_right(&mut self) {
        let ((_, y1), (x2, y2)) = self.panes[self.active_layer][self.active_panes[self.active_layer]].get_corners();

        // We add 1 to make sure that we aren't on the current pane
        let pane_right = x2 + 1;

        let pane_middle = (y1 + y2) / 2;

        let mut pane_index = None;
        // This loop tries to find the pane that is to the right of the current pane
        for (i, pane) in self.panes[self.active_layer].iter().enumerate() {
            let ((x1, y1), (_, y2)) = pane.get_corners();
            if x1 == pane_right && y1 <= pane_middle && pane_middle <= y2 {
                pane_index = Some(i);
                break;
            }
        }

        match pane_index {
            Some(index) => {
                self.active_panes[self.active_layer] = index;
            }
            None => {}
        }
    }

    fn pane_left(&mut self) {
        let ((x1, y1), (_, y2)) = self.panes[self.active_layer][self.active_panes[self.active_layer]].get_corners();

        let pane_left = x1.saturating_sub(1);

        let pane_middle = (y1 + y2) / 2;

        let mut pane_index = None;
        // This loop tries to find the pane that is to the left of the current pane
        for (i, pane) in self.panes[self.active_layer].iter().enumerate() {
            let ((_, y1), (x2, y2)) = pane.get_corners();
            if x2 == pane_left && y1 <= pane_middle && pane_middle <= y2 {
                pane_index = Some(i);
                break;
            }
        }

        match pane_index {
            Some(index) => {
                self.active_panes[self.active_layer] = index;
            }
            None => {}
        }
    }

    fn read_messages(&mut self) -> io::Result<()> {
        match self.channels.1.try_recv() {
            Ok(message) => {
                match message {
                    WindowMessage::HorizontalSplit => {
                        self.horizontal_split();
                        Ok(())
                    }
                    WindowMessage::VerticalSplit => {
                        self.vertical_split();
                        Ok(())
                    }
                    WindowMessage::ForceQuitAll => {
                        for layers in self.panes.iter_mut() {
                            for pane in layers.iter_mut() {
                                pane.close();
                            }
                        }
                        self.editor_sender.send(EditorMessage::Quit).unwrap();
                        Ok(())
                    }
                    WindowMessage::PaneUp => {
                        self.pane_up();
                        Ok(())
                    },
                    WindowMessage::PaneDown => {
                        self.pane_down();
                        Ok(())
                    },
                    WindowMessage::PaneLeft => {
                        self.pane_left();
                        Ok(())
                    },
                    WindowMessage::PaneRight => {
                        self.pane_right();
                        Ok(())
                    },
                    WindowMessage::OpenFile(path, pos) => {
                        self.switch_pane(path, pos)?;
                        self.force_refresh_screen()?;
                        Ok(())
                    }
                    WindowMessage::ClosePane(go_down, uuid) => {

                        match uuid {
                            None => {
                                //self.panes[self.active_layer].remove(self.active_panes[self.active_layer]);
                                self.panes[self.active_layer][self.active_panes[self.active_layer]].close();
                                self.active_panes[self.active_layer] = self.active_panes[self.active_layer].saturating_sub(1);

                                if go_down {
                                    self.active_layer = self.active_layer.saturating_sub(1);
                                }
                            },
                            Some(uuid) => {
                                let coords = self.id_to_pane.get(&uuid).unwrap();

                                self.panes[coords.0][coords.1].close();

                                if go_down {
                                    self.active_layer = coords.0.saturating_sub(1);
                                }
                                self.force_refresh_screen()?;
                            }
                        }

                        for buffer in self.buffers.iter_mut() {
                            buffer.hard_clear();
                        }
                        
                        Ok(())
                    },
                    WindowMessage::CreatePopup(mut container, make_active) => {
                        container.set_move_not_resize(true);
                        self.create_popup(container, make_active);
                        self.force_refresh_screen()?;
                        Ok(())
                    },
                    WindowMessage::OpenNewTab => {
                        self.editor_sender.send(EditorMessage::NewWindow(None)).unwrap();
                        self.skip = true;
                        Ok(())
                    },
                    WindowMessage::OpenNewTabWithPane => {
                        let pane = self.panes[self.active_layer][self.active_panes[self.active_layer]].get_pane();
                        self.panes[self.active_layer][self.active_panes[self.active_layer]].close();

                        self.active_panes[self.active_layer] = self.active_panes[self.active_layer].saturating_sub(1);

                        self.editor_sender.send(EditorMessage::NewWindow(Some(pane))).unwrap();
                        self.skip = true;
                        Ok(())
                    },
                    WindowMessage::NextTab => {
                        self.editor_sender.send(EditorMessage::NextWindow).unwrap();
                        self.skip = true;
                        Ok(())
                    },
                    WindowMessage::PreviousTab => {
                        self.editor_sender.send(EditorMessage::PrevWindow).unwrap();
                        self.skip = true;
                        Ok(())
                    },
                    WindowMessage::NthTab(n) => {
                        self.editor_sender.send(EditorMessage::NthWindow(n)).unwrap();
                        self.skip = true;
                        Ok(())
                    },
                    WindowMessage::PasteResponse(text) => {
                        self.skip = true;

                        match text {
                            None => {},
                            Some(text) => {
                                let command = format!("insert_text {}", text);

                                self.panes[self.active_layer]
                                    [self.active_panes[self.active_layer]].execute_command(&command);
                            }
                        }
                        self.force_refresh_screen()?;
                        
                        Ok(())
                    },
                    WindowMessage::Paste(ty) => {
                        self.skip = true;
                        self.editor_sender.send(EditorMessage::Paste(ty)).unwrap();
                        Ok(())
                    },
                    WindowMessage::Copy(ty, string) => {
                        self.skip = true;
                        self.editor_sender.send(EditorMessage::Copy(ty, string)).unwrap();
                        Ok(())
                    },
                    
                }
            },
            Err(_) => Ok(()),
        }
        
    }

    fn process_event(&mut self) -> io::Result<Event> {
        //self.refresh_screen()?;
        loop {
            if event::poll(self.duration)? {
                return event::read();
            }
        }
    }

    pub fn can_continue(&mut self) -> io::Result<bool> {
        self.read_messages()?;
        self.remove_panes();
        if self.panes[0].len() == 0 {
            eprintln!("No panes left");
            self.editor_sender.send(EditorMessage::CloseWindow).unwrap();
            return Ok(false);
        }

        let ((x1, y1), (x2, y2)) = self.panes[0][self.active_panes[0]].get_corners();

        if x1 == x2 || y1 == y2 {
            eprintln!("Pane is too small");
            //eprintln!("x1: {}, x2: {}, y1: {}, y2: {}", x1, x2, y1, y2);
            self.editor_sender.send(EditorMessage::CloseWindow).unwrap();
            return Ok(false);
        }

        Ok(true)
    }

    pub fn skip_event(&mut self) -> bool {
        let skip = self.skip;
        self.skip = false;
        skip
    }

    pub fn reset_active_pane(&mut self) {
        self.panes[self.active_layer][self.active_panes[self.active_layer]].reset();
    }


    pub fn run(&mut self) -> io::Result<bool> {
        //eprintln!("Running");
        
        //self.refresh_screen()?;
        self.read_messages()?;
        self.remove_panes();
        if self.panes[0].len() == 0 {
            eprintln!("No panes left");
            self.editor_sender.send(EditorMessage::CloseWindow).unwrap();
            return Ok(false);
        }

        self.refresh_screen()?;
        let ((x1, y1), (x2, y2)) = self.panes[0][self.active_panes[0]].get_corners();

        if x1 == x2 || y1 == y2 {
            eprintln!("Pane is too small");
            //eprintln!("x1: {}, x2: {}, y1: {}, y2: {}", x1, x2, y1, y2);
            self.editor_sender.send(EditorMessage::CloseWindow).unwrap();
            return Ok(false);
        }

        if self.skip {
            self.skip = false;
            return Ok(true);
        }

        
        self.panes[self.active_layer][self.active_panes[self.active_layer]].reset();
        
        //eprintln!("Getting Event");
        let event = self.process_event()?;
        match event {
            Event::Key(key) => {
                self.process_keypress(key)
            },
            Event::Resize(width, height) => {
                //self.resize(width, height);

                self.refresh_screen()?;
                
                Ok(true)
            }
            _ => {
                Ok(true)},
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.size = (width as usize, height as usize - 1);
        for pane in self.panes[self.active_layer].iter_mut() {
            pane.resize((width as usize, height as usize));
        }
        for buffer in self.buffers.iter_mut() {
            buffer.resize((width as usize, height as usize - 1));
        }
        self.compositor.resize((width as usize, height as usize - 1));
    }

    pub fn clear_screen() -> io::Result<()> {
        execute!(std::io::stdout(), terminal::Clear(terminal::ClearType::All))?;
        execute!(std::io::stdout(), cursor::MoveTo(0, 0))
    }


    
    fn draw_rows(&mut self) {
        let rows = self.size.1;
        let cols = self.size.0;


        //eprintln!("panes: {}", self.panes.len());
        //let panes = self.panes.len();

        for l in 0..self.panes.len() {
            for i in 0..rows {
                let mut pane_index = 0;
                let mut window_index = 0;
                while window_index < self.size.0 {
                    if pane_index >= self.panes[l].len() {
                        break;
                    }
                    let ((start_x, start_y), (end_x, end_y)) = self.panes[l][pane_index].get_corners();


                    if self.buffers[l].contents.len() <= i {
                        self.buffers[l].contents.push(TextRow::new());
                    }

                    while window_index <= start_x {
                        self.buffers[l].contents[i].push(Some(None));
                        window_index += 1;
                    }
                    
                    if start_y <= i && end_y >= i {

                        /*while window_index < start_x.saturating_sub(1) {
                            self.buffers[l].contents[i].push(None);
                            window_index += 1;
                        }*/

                        /*while self.buffers[l].contents[i].len() < start_x.saturating_sub(1) {
                            self.buffers[l].contents[i].push(None);
                        }*/
                        
                        self.panes[l][pane_index].draw_row(i - start_y, &mut self.buffers[l].contents[i]);
                        window_index += end_x - start_x + 1;
                        
                    }
                    else {
                        if self.buffers[l].contents.len() <= i {
                            let mut text_row = TextRow::new();
                            text_row.extend(vec![None; cols]);
                            self.buffers[l].contents.push(text_row);
                        }
                    }
                    pane_index += 1;
                }

                while self.buffers[l].contents.len() <= i {
                    self.buffers[l].contents.push(TextRow::new());
                }

                while self.buffers[l].contents[i].len() < cols {
                    self.buffers[l].contents[i].push(Some(None));
                }

                let color_settings = &self.settings.borrow().colors.pane;

                if l == 0 {
                    self.buffers[l].contents[i].push(Some(Some(StyledChar::new('\r', color_settings.clone()))));
                    self.buffers[l].contents[i].push(Some(Some(StyledChar::new('\n', color_settings.clone()))));


                }


            }


        }

        self.compositor.merge(&mut self.buffers);

        self.compositor.draw(&mut self.contents);

        self.compositor.clear();
        for buffer in self.buffers.iter_mut() {
            buffer.clear();
        }


    }

    /// This function draws the status bar at the bottom of the screen
    pub fn draw_status_bar(&mut self) {
        //Self::clear_screen().unwrap();
        queue!(
            self.contents,
            terminal::Clear(ClearType::UntilNewLine),
        ).unwrap();

        let settings = self.settings.borrow();
        
        let color_settings = &settings.colors.bar;

        let (name, first, second) = self.panes[0][self.active_panes[0]].get_status();
        let total = name.len() + 1 + first.len() + second.len();// plus one for the space

        let mode_color = &settings.colors.mode.get(&name).unwrap_or(&color_settings);

        self.contents.push_str(apply_colors!(format!("{}", name), mode_color));

        self.contents.push_str(apply_colors!(" ", color_settings));

        self.contents.push_str(apply_colors!(first, color_settings));
        
        let remaining = self.size.0.saturating_sub(total);

        
        self.contents.push_str(apply_colors!(" ".repeat(remaining), color_settings));


        self.contents.push_str(apply_colors!(second, color_settings));
    }

    pub fn force_refresh_screen(&mut self) -> io::Result<()> {
        //Self::clear_screen()?;
        for layer in self.panes.iter_mut() {
            for pane in layer.iter_mut() {
                pane.changed();
            }
        }
        self.refresh_screen()
    }

    pub fn refresh_screen(&mut self) -> io::Result<()> {
        

        /*if !self.contents.will_change() {
            return Ok(());
        }*/

        for layer in self.panes.iter_mut() {
            for pane in layer.iter_mut() {
                pane.refresh();
                pane.scroll_cursor();
            }
        }

        //self.panes[self.active_layer][self.active_panes[self.active_layer]].refresh();

        match self.panes.get_mut(self.active_layer) {
            None => {},
            Some(layer) => {
                match layer.get_mut(self.active_panes[self.active_layer]) {
                    None => {},
                    Some(pane) => {
                        pane.refresh();
                        pane.scroll_cursor();
                    }
                }
            }
        }//[self.active_panes[self.active_layer]].scroll_cursor();

        queue!(
            self.contents,
            cursor::Hide,
            cursor::MoveTo(0, 0),
        )?;

        //eprintln!("drawing rows");
        self.draw_rows();
        //eprintln!("drawing status bar");
        self.draw_status_bar();


        

        let cursor = self.panes[0][self.active_panes[self.active_layer]].get_cursor();
        let cursor = cursor.borrow();

        let (x, y) = cursor.get_real_cursor();
        //eprintln!("x: {} y: {}", x, y);
        let x = x + self.panes[0][self.active_panes[self.active_layer]].get_position().0;
        let y = y + self.panes[0][self.active_panes[self.active_layer]].get_position().1;
        //eprintln!("x: {} y: {}", x, y);

        
        let x = x;// + cursor.number_line_size;

        let (x, y) = if cursor.ignore_offset {
            cursor.get_draw_cursor()
        }
        else {
            (x, y)
        };

        queue!(
            self.contents,
            cursor::MoveTo(x as u16, y as u16),
            cursor::Show,
        )?;


        

        self.contents.flush()
    }

    pub fn open_file_start(&mut self, filename: &str) -> io::Result<()> {
        let pane = self.file_opener(filename.into())?;

        self.panes[0][self.active_panes[0]].change_pane(pane);

        Ok(())
        //self.panes[self.active_layer][self.active_panes[self.active_layer]].open_file(&PathBuf::from(filename.to_owned()))
    }

    pub fn process_keypress(&mut self, key: KeyEvent) -> io::Result<bool> {
        self.panes[self.active_layer][self.active_panes[self.active_layer]].process_keypress(key)
    }

}

#[derive(Clone, PartialEq)]
pub struct StyledChar {
    pub chr: char,
    pub color: ColorScheme,
    pub changed: bool,
}

impl Debug for StyledChar {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}",self.chr)
    }
}

impl StyledChar {
    pub fn new(chr: char, color: ColorScheme) -> Self {
        Self {
            chr,
            color,
            changed: true,
        }
    }

    pub fn style(&self) -> StyledContent<String> {
        apply_colors!(self.chr.to_string(), self.color)
    }
}

#[derive(Clone)]
pub struct TextRow {
    pub contents: Vec<Rc<RefCell<Option<StyledChar>>>>,
    pub index: usize,
    pub changed: bool,
}

impl Debug for TextRow {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.contents)
    }
}

impl TextRow {
    pub fn new() -> TextRow {
        TextRow {
            contents: Vec::new(),
            index: 0,
            changed: false,
        }
    }

    pub fn clear(&mut self) {

        self.index = 0;
        self.changed = false;
    }

    pub fn push(&mut self, chr: Option<Option<StyledChar>>) {
        if self.index >= self.contents.len() {
            match chr {
                None => panic!("Tried to push None on first draw"),
                Some(chr) => self.contents.push(Rc::new(RefCell::new(chr))),
            }
            self.changed = true;
        }
        else {
            match chr {
                None => {
                    self.contents[self.index].borrow_mut().as_mut().unwrap().changed = false;
                },
                Some(chr) => {
                    self.changed = true;
                    self.contents[self.index] = Rc::new(RefCell::new(chr));
                },
            }
        }
        self.index += 1;
    }

    pub fn extend(&mut self, mut other: Vec<Option<StyledChar>>) {
        let mut index = 0;
        while index < self.contents.len() {
            if self.index == self.contents.len() {
                if index >= other.len() {
                    break;
                }
                self.contents.push(Rc::new(RefCell::new(other[index].take())));
            }
            else {
                if index >= other.len() {
                    break;
                }
                self.contents[self.index] = Rc::new(RefCell::new(other[index].take()));
            }
            self.index += 1;
            index += 1;
        }
        //self.contents.extend(other[index..].iter().cloned().map(Rc::new));

        //self.index += other[index..].len();
        self.changed = true;
    }

    pub fn len(&self) -> usize {
        self.contents.len()
    }

    pub fn resize(&mut self, cols: usize) {
        self.contents.truncate(cols);
        self.index = 0;
        self.changed = true;
    }

    pub fn hard_clear(&mut self) {
        self.contents.clear();
        self.index = 0;
        self.changed = true;
    }
    
}

impl Index<usize> for TextRow {
    type Output = Rc<RefCell<Option<StyledChar>>>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.contents[index]
    }
}

#[derive(Clone, Debug)]
pub struct TextBuffer {
    pub contents: Vec<TextRow>,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        //eprintln!("Clearing");
        //eprintln!("{:#?}", self.contents);
        for row in self.contents.iter_mut() {
            row.clear();
        }
    }

    pub fn resize(&mut self, (cols, rows): (usize, usize)) {
        self.contents.truncate(rows);
        for row in self.contents.iter_mut() {
            row.resize(cols);
        }
    }

    pub fn hard_clear(&mut self) {
        for row in self.contents.iter_mut() {
            row.hard_clear();
        }
    }

}

pub struct CompositorRow {
    pub contents: Vec<StyledChar>,
    pub index: usize,
    pub changed: bool,
}

impl Debug for CompositorRow {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.contents)
    }
}

impl CompositorRow {
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
            index: 0,
            changed: false,
        }
    }

    pub fn clear(&mut self) {
        self.index = 0;
        self.changed = false;
    }

    pub fn push(&mut self, chr: Option<StyledChar>) {
        if self.index >= self.contents.len() {
            match chr {
                None => panic!("Tried to push None on first draw"),
                Some(chr) => self.contents.push(chr),
            }
            self.changed = true;
        }
        else {
            match chr {
                None => {},
                Some(chr) => {
                    self.changed = true;
                    self.contents[self.index] = chr;
                },
            }
        }
        self.index += 1;
    }


    pub fn len(&self) -> usize {
        self.contents.len()
    }

    pub fn resize(&mut self, cols: usize) {
        // in the future we may want to only get rid of what has been cut off
        self.contents.truncate(cols);
        self.index = 0;
        self.changed = true;
    }
    
}

impl Index<usize> for CompositorRow {
    type Output = StyledChar;

    fn index(&self, index: usize) -> &Self::Output {
        &self.contents[index]
    }
}


pub struct Compositor {
    pub contents: Vec<CompositorRow>,
    row: usize,
    cols: usize,
    rows: usize,
}

impl Debug for Compositor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:#?}", self.contents)
    }
}

impl Compositor {
    pub fn new((cols, rows): (usize,usize)) -> Self {
        Self {
            contents: Vec::new(),
            row: 0,
            cols,
            rows,
            
        }
    }

    pub fn clear(&mut self) {
        self.row = 0;
        for row in self.contents.iter_mut() {
            row.clear();
        }
    }

    pub fn merge(&mut self, layers: &mut Vec<TextBuffer>) {
        
        let top_layer = layers.len() - 1;

        let min_y = layers.iter().map(|layer| layer.contents.len()).min().unwrap_or(0);
        let min_x = layers.iter().map(|layer| layer.contents.iter().map(|row| row.len()).min().unwrap_or(0)).min().unwrap_or(0);

        for y in 0..min_y {


            for x in 0..min_x {

                let mut curr_layer = top_layer;

                
                while layers[curr_layer].contents[y].len() == 0 || layers[curr_layer].contents[y][x].borrow().is_none() && curr_layer > 0 {
                    curr_layer -= 1;
                }

                if !layers[curr_layer].contents[y].changed {
                    eprintln!("skipping {}", y);
                    continue;
                }

                //let chr_ref = layers[curr_layer].contents[y][x].clone().borrow().clone();

                // For this code to work properly, the first item must be a Some(None) so that we can create a new row if needed
                if layers[curr_layer].contents[y][x].clone().borrow().is_some() {
                    //eprintln!("writing ");
                    //self.contents.push(CompositorRow::new());

                    let chr = layers[curr_layer].contents[y][x].clone();
                    let mut chr = chr.borrow_mut();

                    let chr = chr.as_mut().unwrap();
                    
                    if chr.changed {
                        self.contents[y].push(Some(chr.clone()));
                        chr.changed = false;
                    }
                    else {
                        self.contents[y].push(None);
                        chr.changed = false;
                        //layers[curr_layer].contents[y][x].borrow_mut().as_mut().unwrap().changed = false;
                    }
                    //self.contents[y].push(Some(chr));
                    //layers[curr_layer].contents[y].changed = false;
                }
                else {
                    if self.contents.len() <= y {
                        //eprintln!("Pushing row");
                        self.contents.push(CompositorRow::new());
                    }
                    //self.contents[y].push(StyledChar::new(' ', ColorScheme::default()));
                }
            }
        }
    }

    pub fn draw(&self, output: &mut WindowContents) {

        let rows = cmp::min(self.rows, self.contents.len());
        let cols = cmp::min(self.cols, self.contents[0].len());
        
        for y in 0..rows {
            for x in 0..cols {
                output.push_str(self.contents[y][x].style());
            }
        }
        //eprintln!("Cols: {}, Rows: {}", cols, rows);
        //eprintln!("{:#?}", self);
    }

    pub fn resize(&mut self, (cols, rows): (usize, usize)) {
        for row in self.contents.iter_mut() {
            row.resize(cols);
        }
        self.cols = cols;
        self.rows = rows;
    }
}




pub trait WindowContentsUtils<T> {
    fn push_str(&mut self, s: T);
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

    fn merge(&mut self, other: &mut Self) {
        self.content.push_str(other.content.as_str());
        other.content.clear();
    }
}

impl WindowContentsUtils<&str> for WindowContents {
    fn push_str(&mut self, s: &str) {
        self.content.push_str(s);
    }
}

impl WindowContentsUtils<String> for WindowContents {
    fn push_str(&mut self, s: String) {
        self.content.push_str(s.as_str());
    }
}

impl WindowContentsUtils<&String> for WindowContents {
    fn push_str(&mut self, s: &String) {
        self.content.push_str(s.as_str());
    }
}

impl WindowContentsUtils<StyledContent<&str>> for WindowContents {
    fn push_str(&mut self, s: StyledContent<&str>) {
        self.content.push_str(&format!("{}", s));
    }
}

impl WindowContentsUtils<StyledContent<String>> for WindowContents {
    fn push_str(&mut self, s: StyledContent<String>) {
        self.content.push_str(&format!("{}", s));
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


