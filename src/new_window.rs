use std::{cell::RefCell, rc::Rc, sync::mpsc::{Receiver, Sender}, collections::HashMap, path::PathBuf, io};

use uuid::Uuid;

use crate::{new_editor::TextLayer, editor::{RegisterType, EditorMessage}, settings::Settings, lsp::LspControllerMessage, treesitter::tree_sitter_scheme};






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




pub struct Window {

    settings: Rc<RefCell<Settings>>,
    active_panes: Vec<usize>,
    active_layer: usize,
    panes: Vec<Vec<PaneContainer>>,
    id_to_pane: HashMap<Uuid, (usize, usize)>,
    text_layers: Vec<TextLayer>,
    /// The receiver for messages for the window to handle
    mailbox: Receiver<WindowMessage>,
    /// The sender for messages to the window
    /// This is to be shared with the panes
    sender: Sender<WindowMessage>,
    editor_sender: Sender<EditorMessage>,
    lsp_sender: Sender<LspControllerMessage>,
    /// The receiver is how we get a response from the lsp,
    lsp_listener: Rc<Receiver<LspControllerMessage>>,
}


impl Window {
    pub fn new(editor_sender: Sender<EditorMessage>,
               lsp_sender: Sender<LspControllerMessage>,
               lsp_listener: Rc<Receiver<LspControllerMessage>>,
               settings: Rc<RefCell<Settings>>) -> Self {

        let (sender, mailbox) = std::sync::mpsc::channel();

        let panes = Vec::new();

        let text_layers = Vec::new();

        let active_panes = Vec::new();

        let active_layer = 0;

        let id_to_pane = HashMap::new();

        Self {
            settings,
            active_panes,
            active_layer,
            panes,
            id_to_pane,
            text_layers,
            mailbox,
            sender,
            editor_sender,
            lsp_sender,
            lsp_listener,
        }

    }

    pub fn get_sender(&self) -> Sender<WindowMessage> {
        self.sender.clone()
    }

    fn file_opener(&mut self, path: PathBuf) -> io::Result<(usize, usize)> {

        let filename = path.file_name().unwrap().to_str();

        if let Some(filename) = filename {

            for layer in 0..self.panes.len() {
                for (index, container) in self.panes[self.active_panes[layer]].iter().enumerate() {
                    if container.get_name() == filename {
                        return Ok((layer, index));
                    }

                }
            }

        }


        let mut pane = TextPane::new(self.settings.clone(),
                                     self.sender.clone(),
                                     self.editor_sender.clone(),
                                     self.lsp_sender.clone(),
                                     self.lsp_listener.clone());

        pane.open_file(filename)?;

        let pane = Rc::new(RefCell::new(pane));


        let pos = self.insert_pane(pane);

        Ok(pos)
    }
    
    fn insert_pane(&mut self, pane: Rc<RefCell<dyn Pane>>) -> (usize, usize) {

        let mut container = if self.panes.len() == 0 {
            self.panes.push(Vec::new());
            self.active_panes.push(0);

            let size = self.settings.borrow().get_window_size();

            let mut container = PaneContainer::new(size, pane, self.settings.clone());

            container.set_size(size);
            
            container
        } else {
            let size = self.settings.borrow().get_window_size();

            let mut container = PaneContainer::new(size, pane, self.settings.clone());

            container.set_size((0, 0));
            
            container

        };

        self.panes[self.active_panes[self.active_layer]].push(container);

        let pos = (self.active_layer, self.panes[self.active_panes[self.active_layer]].len() - 1);

        self.id_to_pane.insert(container.get_id(), pos);

        pos
    }

    pub fn open_file(&mut self, filename: PathBuf) -> io::Result<()> {
        let pane: Rc<RefCell<dyn Pane>> = self.file_opener(filename)?;

        let pos = self.insert_pane(pane);


        self.switch_pane(pos, None);

        Ok(())
    }

    pub fn open_file_at(&mut self, filename: PathBuf, location: (usize, usize)) -> io::Result<()> {
        let pos = self.file_opener(filename)?;

        self.switch_pane(pos, Some(location));

        Ok(())
    }


    fn switch_pane(&mut self, (layer, index): (usize, usize), location: Option<(usize, usize)>) {

        let new_active_container = &mut self.panes[self.active_panes[layer]][index];

        let old_active_container = &mut self.panes[self.active_panes[self.active_layer]][self.active_panes[self.active_layer]];


        let active_pane = new_active_container.get_pane().clone();
        let new_active_pane = new_active_container.get_pane().clone();

        active_pane.borrow_mut().reset();

        old_active_container.change_pane(new_active_pane);
        new_active_container.change_pane(active_pane);



        if let Some(location) = location {
           old_active_container.set_location(location);
        }
    }



}
