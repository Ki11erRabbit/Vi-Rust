use std::{cell::RefCell, rc::Rc, sync::mpsc::{Receiver, Sender}, collections::HashMap, path::PathBuf};

use uuid::Uuid;

use crate::{new_editor::TextLayer, editor::{RegisterType, EditorMessage}, settings::Settings, lsp::LspControllerMessage};






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

        let active_panes = vec![0];

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


    pub fn open_file(&mut self, filename: PathBuf) -> io::Result<()> {
        let pane: Rc<RefCell<dyn Pane>> = self.file_opener(filename)?;

        let pos = self.insert_pane(pane);


        self.switch_pane(pos, None);

        Ok(())
    }

    pub fn open_file_at(&mut self, filename: PathBuf, location: (usize, usize)) -> io::Result<()> {
        let pane: Rc<RefCell<dyn Pane>> = self.file_opener(filename)?;

        let pos = self.insert_pane(pane);

        self.switch_pane(pos, Some(location));

        Ok(())
    }

}
