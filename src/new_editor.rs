
use std::io::Write;
use std::path::PathBuf;
use std::{cmp, io};
use std::fmt::{Formatter, self};
use std::ops::{Index, IndexMut};
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::mpsc::{Sender, Receiver};
use std::time::Duration;

use crossterm::cursor::{SetCursorStyle, MoveTo, Hide, Show};
use crossterm::event::{Event, KeyEvent};
use crossterm::style::StyledContent;
use crossterm::terminal::ClearType;
use crossterm::{terminal, execute, event, queue};

use crate::lsp::LspControllerMessage;
use crate::new_window::WindowMessage;
use crate::{apply_colors, Mailbox};
use crate::new_pane::Pane;
use crate::registers::Registers;
use crate::settings::{ColorScheme, Settings};
use crate::new_window::Window;




pub enum EditorMessage {
    NextWindow,
    PrevWindow,
    NewWindow(Option<Rc<RefCell<dyn Pane>>>),
    CloseWindow,
    Quit,
    NthWindow(usize),
    Paste(RegisterType),
    Copy(RegisterType, String),
}

#[derive(Clone, Debug)]
pub enum RegisterType {
    Number(usize),
    Name(String),
    None,
}



pub struct EditorMailbox {
    /// The receiver for messages to the editor
    local_receiver: Receiver<EditorMessage>,
    /// The sender for messages to the editor
    /// This isn't wrapped in an Rc because it is easy to share
    far_sender: Sender<EditorMessage>,
    /// The receiver for messages not to the editor
    /// This is wrapped in an Rc so that it can be shared with other parts of the editor
    far_receiver: Rc<Receiver<EditorMessage>>,
    /// The sender for messages not to the editor
    local_sender: Sender<EditorMessage>,
}

impl EditorMailbox {
    pub fn new() -> Self {
        let (local_sender, local_receiver) = std::sync::mpsc::channel();
        let (far_sender, far_receiver) = std::sync::mpsc::channel();

        Self {
            local_receiver,
            far_sender,
            far_receiver: Rc::new(far_receiver),
            local_sender,
        }
    }

    pub fn get_far_receiver(&self) -> Rc<Receiver<EditorMessage>> {
        self.far_receiver.clone()
    }

    pub fn get_far_sender(&self) -> Sender<EditorMessage> {
        self.far_sender.clone()
    }

}

impl Mailbox<EditorMessage> for EditorMailbox {
    fn send(&self, message: EditorMessage) -> Result<(), std::sync::mpsc::SendError<EditorMessage>> {
        self.local_sender.send(message)
    }
    

    fn recv(&self) -> Result<EditorMessage, std::sync::mpsc::RecvError> {
        self.local_receiver.recv()
    }

    fn try_recv(&self) -> Result<EditorMessage, std::sync::mpsc::TryRecvError> {
        self.local_receiver.try_recv()
    }


}


pub struct Editor {
    windows: Vec<Window>,
    window_senders: Vec<Sender<WindowMessage>>,
    active_window: usize,
    mailbox: EditorMailbox,

    /// The receiver for messages from the LSP controller
    /// It is wrapped in an Rc so that it can be shared with other parts of the editor
    lsp_listener: Rc<Receiver<LspControllerMessage>>,
    /// The sender for messages to the LSP controller
    /// This isn't wrapped in an Rc because it is easy to share
    lsp_sender: Sender<LspControllerMessage>,

    /// The registers for holding text.
    registers: Registers,

    settings: Rc<RefCell<Settings>>,
    poll_duration: Duration,

    compositor: Compositor,
    output_buffer: OutputBuffer,
}


impl Editor {
    pub fn new(lsp_sender: Sender<LspControllerMessage>, lsp_listener: Rc<Receiver<LspControllerMessage>>) -> Self {
        terminal::enable_raw_mode().expect("Failed to enable raw mode");
        execute!(std::io::stdout(), terminal::EnterAlternateScreen).expect("Failed to enter alternate screen");
        execute!(std::io::stdout(), SetCursorStyle::BlinkingBlock).expect("Failed to set cursor style");


        let size = terminal::size()
            .map(|(w, h)| (w as usize, h as usize)).unwrap();

        let mailbox = EditorMailbox::new();


        //todo: load settings from file
        let mut settings = Settings::default();

        settings.cols = size.0;
        settings.rows = size.1;

        let poll_duration = Duration::from_millis(settings.editor_settings.poll_timeout);



        let settings = Rc::new(RefCell::new(settings));

        let window = Window::new(mailbox.get_far_sender(),
                                 mailbox.get_far_receiver(),
                                 lsp_sender.clone(),
                                 lsp_listener.clone(),
                                 settings.clone());


        let window_sender = window.get_sender();


        Self {
            windows: vec![window],
            window_senders: vec![window_sender],
            active_window: 0,
            mailbox,
            lsp_listener,
            lsp_sender,
            registers: Registers::new(),
            settings,
            compositor: Compositor::new(size),
            output_buffer: OutputBuffer::new(size),
            poll_duration,
        }

    }


    #[inline]
    fn write(&mut self) {
        self.windows[self.active_window].draw(&mut self.compositor);
    }
    

    /// This function draws the current window to the compositor
    fn draw(&mut self) {

        self.write();
        self.compositor.draw(&mut self.output_buffer);
        //self.compositor.merge(&mut self.text_layers);
    }


    fn get_cursor_coords(&self) -> Option<(usize, usize)> {
        self.windows[self.active_window].get_cursor_coords()
    }

    pub fn clear_screen() -> io::Result<()> {

        queue!(
            std::io::stdout(),
            terminal::Clear(ClearType::UntilNewLine),
        ).unwrap();
        //execute!(std::io::stdout(), terminal::Clear(terminal::ClearType::All))
        //execute!(std::io::stdout(), cursor::MoveTo(0, 0))
        Ok(())
    }

    /// This function is called every time we want to redraw the screen
    /// We also move or hide the cursor here
    fn refresh_screen(&mut self) -> io::Result<()> {

        
        self.windows[self.active_window].refresh();

        queue!(
            self.output_buffer,
            Hide,
            MoveTo(0, 0),
        )?;

        Self::clear_screen()?;
        self.draw();
        self.output_buffer.flush()?;

        let cursor = self.get_cursor_coords();

        if let Some((x, y)) = cursor {
            queue!(
                self.output_buffer,
                MoveTo(x as u16, y as u16),
                Show,
            )?;
        }

        
        Ok(())
    }

    fn process_event(&mut self) -> io::Result<Event> {
        loop {
            if event::poll(self.poll_duration)? {
                return event::read();
            }
        }
    }


    fn resize(&mut self, cols: usize, rows: usize) {
        for window in &mut self.windows {
            window.resize((cols, rows));
        }
    }

    fn process_keypress(&mut self, key: KeyEvent) -> io::Result<()> {
        self.windows[self.active_window].process_keypress(key)
    }


    pub fn run(&mut self) -> io::Result<bool> {
        if self.windows[self.active_window].can_close()? {
            self.windows.remove(self.active_window);
            self.window_senders.remove(self.active_window);
            self.active_window = self.active_window.saturating_sub(1);
        }

        if self.windows.is_empty() {
            return Ok(false);
        }



        self.check_messages();


        self.refresh_screen()?;


        let event = self.process_event()?;

        match event {
            Event::Key(key) => {
                self.process_keypress(key)?;
                Ok(true)
            },
            Event::Resize(w, h) => {
                self.resize(w as usize, h as usize);

                self.refresh_screen()?;
                Ok(true)
            },
            _ => Ok(true),
        }


    }


    fn check_messages(&mut self) {

    }

    pub fn open_file(&mut self, path: &str) -> io::Result<()> {
        self.windows[self.active_window].first_open(PathBuf::from(path))
    }

}

impl Drop for Editor {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Failed to disable raw mode");
        execute!(std::io::stdout(), terminal::Clear(terminal::ClearType::All)).expect("Failed to clear terminal");
        execute!(std::io::stdout(), MoveTo(0, 0)).expect("Failed to move cursor to 0, 0");
        execute!(std::io::stdout(), terminal::LeaveAlternateScreen).expect("Failed to leave alternate screen");
        execute!(io::stdout(), SetCursorStyle::DefaultUserShape).expect("Could not reset cursor style");
    }
}




#[derive(Clone, PartialEq)]
pub struct StyledChar {
    pub chr: char,
    pub color: ColorScheme,
    pub changed: bool,
}

impl core::fmt::Debug for StyledChar {
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
pub struct LayerRow {
    pub contents: Vec<Rc<RefCell<Option<StyledChar>>>>,
    pub index: usize,
    pub changed: bool,
}

impl core::fmt::Debug for LayerRow {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.contents)
    }
}

impl LayerRow {
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
            index: 0,
            changed: true,
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

impl Index<usize> for LayerRow {
    type Output = Rc<RefCell<Option<StyledChar>>>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.contents[index]
    }
}

#[derive(Clone, Debug)]
pub struct TextLayer {
    pub contents: Vec<LayerRow>,
}


impl Index<usize> for TextLayer {
    type Output = LayerRow;

    fn index(&self, index: usize) -> &Self::Output {
        &self.contents[index]
    }
}

impl IndexMut<usize> for TextLayer {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.contents[index]
    }
}

impl TextLayer {
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

impl core::fmt::Debug for CompositorRow {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.contents)
    }
}

impl CompositorRow {
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
            index: 0,
            changed: true,
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

impl core::fmt::Debug for Compositor {
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

    pub fn merge(&mut self, layers: &mut Vec<TextLayer>) {
        
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
                    continue;
                }

                //let chr_ref = layers[curr_layer].contents[y][x].clone().borrow().clone();
                
                if layers[curr_layer].contents[y][x].clone().borrow().is_some() {
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
                        self.contents.push(CompositorRow::new());
                    }
                    //self.contents[y].push(StyledChar::new(' ', ColorScheme::default()));
                }
            }
        }
    }

    pub fn draw(&mut self, output: &mut OutputBuffer) {

        let rows = cmp::min(self.rows, self.contents.len());
        let cols = cmp::min(self.cols, self.contents[0].len());
        
        for y in 0..rows {
            for x in 0..cols {
                output.push(self.contents[y][x].style());
            }
        }
        //eprintln!("Cols: {}, Rows: {}", cols, rows);
        //eprintln!("{:#?}", self);
        self.clear();
    }

    pub fn resize(&mut self, (cols, rows): (usize, usize)) {
        for row in self.contents.iter_mut() {
            row.resize(cols);
        }
        self.cols = cols;
        self.rows = rows;
    }
}


#[derive(Debug, Clone)]
pub struct OutputRow {
    contents: Vec<String>,
    index: usize,
}

impl Default for OutputRow {
    fn default() -> Self {
        Self {
            contents: Vec::new(),
            index: 0,
        }
    }
}

impl OutputRow {
    pub fn new(cols: usize) -> Self {
        Self {
            contents: vec![" ".to_string(); cols],
            index: 0,
        }
    }

    pub fn clear(&mut self) {
        self.index = 0;
    }

    pub fn push(&mut self, chr: Option<StyledContent<String>>) -> bool {
        if self.index >= self.contents.len() {
            match chr {
                None => panic!("Tried to push None on first draw"),
                Some(chr) => self.contents.push(chr.to_string()),
            }
        }
        else {
            match chr {
                None => {},
                Some(chr) => {
                    self.contents[self.index] = chr.to_string();
                },
            }
        }

        let contains_newline = self.contents[self.index].contains('\n');
        
        self.index += 1;

        contains_newline
    }

    pub fn resize(&mut self, cols: usize) {
        self.contents.truncate(cols);
        self.index = 0;
    }

    pub fn write(&self) -> io::Result<()> {
        for chr in self.contents.iter() {
            write!(std::io::stdout(), "{}", chr)?;
        }
        Ok(())
    }

}


impl Index<usize> for OutputRow {
    type Output = String;

    fn index(&self, index: usize) -> &Self::Output {
        &self.contents[index]
    }
}

pub struct OutputBuffer {
    //contents: Vec<OutputRow>,
    contents: String,
}


impl OutputBuffer {
    pub fn new((cols, rows): (usize, usize)) -> Self {
        Self {
            //contents: vec![OutputRow::new(cols); rows],
            contents: String::new(),
        }

    }

    pub fn clear(&mut self) {
        self.contents.clear();
    }

    pub fn push(&mut self, content: StyledContent<String>) {

        self.contents.push_str(&content.to_string());
        /*if self.current_row >= self.contents.len() {
            self.contents.push(OutputRow::default());
            if self.contents[self.current_row].push(content) {
                self.current_row += 1;
            }
        }
        else {
            if self.contents[self.current_row].push(content) {
                self.current_row += 1;
            }
        }*/
    }

    pub fn resize(&mut self, (cols, rows): (usize, usize)) {
        /*for row in self.contents.iter_mut() {
            row.resize(cols);
        }
        self.contents.truncate(rows);*/
    }
}


impl io::Write for OutputBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                Ok(buf.len())
            }
            Err(_) => Err(io::ErrorKind::WriteZero.into()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut out = Ok(());
        /*for row in self.contents.iter() {
            out = row.write();

            if out.is_err() {
                return out;
            }
    }*/
        write!(std::io::stdout(), "{}", self.contents)?;
        
        std::io::stdout().flush()?;
        self.clear();
        out
    }

}
