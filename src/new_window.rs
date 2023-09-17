use std::{cell::RefCell, rc::Rc, sync::mpsc::{Receiver, Sender}, collections::HashMap, path::PathBuf, io, cmp};

use crossterm::event::KeyEvent;
use uuid::Uuid;

use crate::{new_editor::{TextLayer, Compositor, StyledChar}, new_editor::{RegisterType, EditorMessage, LayerRow}, settings::Settings, lsp::LspControllerMessage,  Mailbox, new_pane::{PaneContainer, text_pane::TextPane, Pane, TextBuffer}};






pub enum WindowMessage {
    HorizontalSplit,
    VerticalSplit,
    ForceQuitAll,
    PaneUp,
    PaneDown,
    PaneLeft,
    PaneRight,
    OpenFile(String, Option<(usize, usize)>),
    /// bool is for whether or not to go down a layer, The Uuid is to make sure we don't close the wrong pane
    ClosePane(bool, Option<Uuid>),
    CreatePopup(PaneContainer, bool),
    OpenNewTabWithPane,
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
        let (local_sender, far_receiver) = std::sync::mpsc::channel();
        let (far_sender, local_receiver) = std::sync::mpsc::channel();

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


pub struct Window {

    settings: Rc<RefCell<Settings>>,
    active_panes: Vec<usize>,
    active_layer: usize,
    panes: Vec<Vec<PaneContainer>>,
    id_to_pane: HashMap<Uuid, (usize, usize)>,
    text_layers: Vec<TextLayer>,
    mailbox: WindowMailbox,
    editor_sender: Sender<EditorMessage>,
    editor_listener: Rc<Receiver<EditorMessage>>,
    lsp_sender: Sender<LspControllerMessage>,
    /// The receiver is how we get a response from the lsp,
    lsp_listener: Rc<Receiver<LspControllerMessage>>,
}


impl Window {
    pub fn new(editor_sender: Sender<EditorMessage>,
               editor_listener: Rc<Receiver<EditorMessage>>,
               lsp_sender: Sender<LspControllerMessage>,
               lsp_listener: Rc<Receiver<LspControllerMessage>>,
               settings: Rc<RefCell<Settings>>) -> Self {

        let mailbox = WindowMailbox::new();

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
            editor_sender,
            editor_listener,
            lsp_sender,
            lsp_listener,
        }

    }

    pub fn lose_focus(&mut self) {

        for layer in self.panes.iter_mut() {
            for container in layer.iter_mut() {
                container.lose_focus();
            }
        }
    }

    pub fn get_sender(&self) -> Sender<WindowMessage> {
        self.mailbox.get_far_sender()
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
                                     self.mailbox.get_far_sender(),
                                     self.mailbox.get_far_receiver(),
                                     self.editor_sender.clone(),
                                     self.editor_listener.clone(),
                                     self.lsp_sender.clone(),
                                     self.lsp_listener.clone());

        pane.open_file(path)?;

        eprintln!("Opened file");

        let pane = Rc::new(RefCell::new(pane));


        let pos = self.insert_pane(pane);

        Ok(pos)
    }
    
    fn insert_pane(&mut self, pane: Rc<RefCell<dyn Pane>>) -> (usize, usize) {

        eprintln!("Inserting pane");
        
        let container = if self.panes.len() == 0 {
            self.text_layers.push(TextLayer::new());
            self.panes.push(Vec::new());
            self.active_panes.push(0);

            let size = self.settings.borrow().get_window_size();

            let mut container = PaneContainer::new(pane, self.settings.clone());

            container.set_max_size(size);
            container.set_size(size);
            
            container
        } else {
            let size = self.settings.borrow().get_window_size();

            let mut container = PaneContainer::new(pane, self.settings.clone());

            container.set_max_size(size);
            container.set_size((0,0));
            
            container

        };

        let id = container.get_id();

        self.panes[self.active_panes[self.active_layer]].push(container);

        let pos = (self.active_layer, self.panes[self.active_panes[self.active_layer]].len() - 1);

        self.id_to_pane.insert(id, pos);

        pos
    }

    pub fn first_open(&mut self, filename: PathBuf) -> io::Result<()> {
        eprintln!("First open: {:?}", filename);
        let _pos = self.file_opener(filename)?;

        Ok(())
    }


    pub fn open_file(&mut self, filename: PathBuf) -> io::Result<()> {
        let pos = self.file_opener(filename)?;

        self.switch_pane(pos, None);

        Ok(())
    }

    pub fn open_file_at(&mut self, filename: PathBuf, location: (usize, usize)) -> io::Result<()> {
        let pos = self.file_opener(filename)?;

        self.switch_pane(pos, Some(location));

        Ok(())
    }


    fn switch_pane(&mut self, (layer, index): (usize, usize), location: Option<(usize, usize)>) {
        eprintln!("Switching to pane: {}, {}", layer, index);


        let active_pane = self.panes[self.active_panes[self.active_layer]]
            [self.active_panes[self.active_layer]]
            .get_pane()
            .clone();
        let new_active_pane = self.panes[self.active_panes[layer]][index].get_pane().clone();

        active_pane.borrow_mut().reset();

        self.panes[self.active_panes[self.active_layer]]
            [self.active_panes[self.active_layer]]
            .change_pane(new_active_pane);
        self.panes[self.active_panes[layer]][index].change_pane(active_pane);



        if let Some(location) = location {
           self.panes[self.active_panes[self.active_layer]]
            [self.active_panes[self.active_layer]].set_location(location);
        }
    }

    pub fn remove_panes(&mut self) {
        let mut panes_to_remove = Vec::new();
        for (i, layer) in self.panes.iter().enumerate() {
            for (j, container) in layer.iter().enumerate() {
                if container.can_close() {
                    eprintln!("removing pane");
                    panes_to_remove.push((i, j));
                }
            }
        }


        for (layer, index) in panes_to_remove.iter().rev() {

            loop {
                if *layer + 1 < self.panes[*index].len() {
                    let corners = self.panes[*layer][*index].get_corners();
                    if self.panes[*layer][*index + 1].combine(corners) {
                        break;
                    }
                }
                if *index != 0 {
                    let corners = self.panes[*layer][*index].get_corners();
                    if self.panes[*layer][*index - 1].combine(corners) {
                        break;
                    }
                    
                }
                break;
            }

            self.panes[*layer].remove(*index);
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
                self.id_to_pane.insert(pane.get_id(), (i, j));
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

        self.id_to_pane.insert(self.panes[self.active_layer][new_pane_index].get_id(), (self.active_layer, new_pane_index));

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

        self.id_to_pane.insert(self.panes[self.active_layer][new_pane_index].get_id(), (self.active_layer, new_pane_index));

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

    fn force_quit_all(&mut self) {
        self.panes.clear();
        self.active_panes.clear();
        self.active_layer = 0;
        self.panes.push(Vec::new());
        self.active_panes.push(0);
    }

    fn close_pane(&mut self, lose_focus: bool, uuid: Option<Uuid>) {
        match uuid {
            None => {
                self.panes[self.active_layer][self.active_panes[self.active_layer]].close();
                
                self.active_panes[self.active_layer] = self.active_panes[self.active_layer].saturating_sub(1);
            },
            Some(uuid) => {
                let coords = self.id_to_pane.get(&uuid).unwrap();

                self.panes[coords.0][coords.1].close();
            },
        }

        self.text_layers[self.active_layer].hard_clear();

        if lose_focus {
            self.active_layer = 0;
        }
        
    }

    fn open_new_tab_with_pane(&mut self) {
        let pane = self.panes[self.active_layer][self.active_panes[self.active_layer]].get_pane();
        self.panes[self.active_layer][self.active_panes[self.active_layer]].close();

        self.active_panes[self.active_layer] = self.active_panes[self.active_layer].saturating_sub(1);

        self.editor_sender.send(EditorMessage::NewWindow(Some(pane))).unwrap();
    }
    

    fn check_messages(&mut self) -> io::Result<bool> {
        let result = self.check_window_messages()?;
        //result = result || self.check_window_messages()?;

        Ok(result)
    }


    fn check_window_messages(&mut self) -> io::Result<bool> {
        match self.mailbox.try_recv() {
            Ok(message) => {
                match message {
                    WindowMessage::HorizontalSplit => {
                        self.horizontal_split();
                        Ok(true)
                    },
                    WindowMessage::VerticalSplit => {
                        self.vertical_split();
                        Ok(true)
                    },
                    WindowMessage::ForceQuitAll => {
                        self.force_quit_all();
                        Ok(false)
                    },
                    WindowMessage::PaneUp => {
                        self.pane_up();
                        Ok(true)
                    },
                    WindowMessage::PaneDown => {
                        self.pane_down();
                        Ok(true)
                    },
                    WindowMessage::PaneLeft => {
                        self.pane_left();
                        Ok(true)
                    },
                    WindowMessage::PaneRight => {
                        self.pane_right();
                        Ok(true)
                    },
                    WindowMessage::OpenFile(path, pos) => {
                        let path = PathBuf::from(path);

                        match pos {
                            None => self.open_file(path)?,
                            Some(pos) => self.open_file_at(path, pos)?,
                        }
                        Ok(true)
                    },
                    WindowMessage::ClosePane(go_down, uuid) => {
                        self.close_pane(go_down, uuid);
                        Ok(true)
                    },
                    WindowMessage::CreatePopup(container, make_active) => {
                        //self.create_popup(container, make_active);
                        Ok(true)
                    },
                    WindowMessage::OpenNewTabWithPane => {
                        self.open_new_tab_with_pane();
                        Ok(true)
                    },
                }


            },
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Ok(false)
            },
            Err(_) => {
                Ok(true)
            }


        }


    }

    pub fn resize(&mut self, size: (usize, usize)) {
        for layer in self.panes.iter_mut() {
            for pane in layer.iter_mut() {
                pane.resize(size);
            }
        }

        for layer in self.text_layers.iter_mut() {
            layer.resize(size);
        }
        
    }

    #[inline]
    fn draw_rows(&mut self) {
        let (cols, rows) = self.settings.borrow().get_window_size();

        //let rows = rows.saturating_sub(1);


        for l in 0..self.panes.len() {
            for i in 0..rows {
                let mut pane_index = 0;
                let mut window_index = 0;
                while window_index < cols {
                    if pane_index >= self.panes[l].len() {
                        break;
                    }
                    let ((start_x, start_y), (end_x, end_y)) = self.panes[l][pane_index].get_corners();


                    if self.text_layers[l].contents.len() <= i {
                        self.text_layers[l].contents.push(LayerRow::new());
                    }

                    while window_index <= start_x {
                        self.text_layers[l].contents[i].push(Some(None));
                        window_index += 1;
                    }
                    
                    if start_y <= i && end_y >= i {
                        
                        self.panes[l][pane_index].draw_row(i - start_y, &mut self.text_layers[l].contents[i]);
                        window_index += end_x - start_x + 1;
                        
                    }
                    else {
                        if self.text_layers[l].contents.len() <= i {
                            let mut text_row = LayerRow::new();
                            text_row.extend(vec![None; cols]);
                            self.text_layers[l].contents.push(text_row);
                        }
                    }
                    pane_index += 1;
                }

                while self.text_layers[l].contents.len() <= i {
                    self.text_layers[l].contents.push(LayerRow::new());
                }

                while self.text_layers[l].contents[i].len() < cols {
                    self.text_layers[l].contents[i].push(Some(None));
                }

                let color_settings = &self.settings.borrow().colors.pane;

                if l == 0 {
                    self.text_layers[l].contents[i].push(Some(Some(StyledChar::new('\r', color_settings.clone()))));
                    self.text_layers[l].contents[i].push(Some(Some(StyledChar::new('\n', color_settings.clone()))));
                }


            }


        }

    }

    fn draw_status_bar(&mut self) {

        let current_layer = self.panes[self.active_layer][self.active_panes[self.active_layer]].draw_status();

        let (name, first, second) = if current_layer {
            self.panes[self.active_layer][self.active_panes[self.active_layer]].get_status()
        } else {
            self.panes[0][self.active_panes[0]].get_status()
        };

        let total = name.len() + 1 + first.len() + second.len();// + 1 for the space between the name and first

        let color_settings = self.settings.borrow().colors.bar.clone();

        let (cols, rows) = self.settings.borrow().get_window_size();

        let row = rows.saturating_sub(2);

        let settings = self.settings.borrow();

        let mode_color = &settings.colors.mode.get(&name).unwrap_or(&color_settings);

        for chr in name.chars() {
            self.text_layers[0][row].push(Some(Some(StyledChar::new(chr, Clone::clone(&mode_color)))));
        }
        self.text_layers[0].contents[row].push(Some(Some(StyledChar::new(' ', color_settings.clone()))));

        for chr in first.chars() {
            self.text_layers[0][row].push(Some(Some(StyledChar::new(chr, color_settings.clone()))));
        }

        for chr in " ".repeat(cols - total).chars() {
            self.text_layers[0][row].push(Some(Some(StyledChar::new(chr, color_settings.clone()))));
        }

        for chr in second.chars() {
            self.text_layers[0][row].push(Some(Some(StyledChar::new(chr, color_settings.clone()))));
        }
    }

    fn write(&mut self) {
        eprintln!("{}", self.panes.len());
        self.draw_rows();

        self.draw_status_bar();
    }


    pub fn draw(&mut self, compositor: &mut Compositor) {

        self.write();
        compositor.merge(&mut self.text_layers);
    }


    pub fn get_cursor_coords(&self) -> Option<(usize, usize)> {

        self.panes[self.active_layer][self.active_panes[self.active_layer]].get_cursor_coords()
    }

    pub fn process_keypress(&mut self, key: KeyEvent) -> io::Result<()> {
        self.panes[self.active_layer][self.active_panes[self.active_layer]].process_keypress(key)
    }

    pub fn can_close(&mut self) -> io::Result<bool> {

        self.remove_panes();

        if !self.check_messages()? {
            eprintln!("Received a message that would cause a close");
            return Ok(true);
        } 

        if self.panes.len() == 0 {
            return Ok(true);
        }

        if self.panes[0].len() == 0 {
            return Ok(true);
        }

        let ((x1, y1), (x2, y2)) = self.panes[0][self.active_panes[0]].get_corners();

        if x1 == x2 && y1 == y2 {
            return Ok(true);
        } else {
            return Ok(false);
        }
        
    }

    pub fn refresh(&mut self) {
        for layer in self.panes.iter_mut() {
            for pane in layer.iter_mut() {
                pane.refresh();
            }
        }
    }
}
