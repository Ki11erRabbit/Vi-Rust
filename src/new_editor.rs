
use std::io::Write;
use std::{cmp, io};
use std::fmt::{Formatter, self};
use std::ops::Index;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::mpsc::{Sender, Receiver};
use std::time::Duration;

use crossterm::cursor::SetCursorStyle;
use crossterm::style::StyledContent;
use crossterm::{terminal, execute};

use crate::apply_colors;
use crate::pane::Pane;
use crate::registers::Registers;
use crate::settings::{ColorScheme, Settings};
use crate::window::Window;




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




pub struct Editor {
    size: (usize, usize),
    
    windows: Vec<Window>,
    window_senders: Vec<Sender<WindowMessage>>,
    active_window: usize,
    /// The receiver for messages to the editor
    mailbox: Receiver<EditorMessage>,
    /// Then sender for messages to the editor
    /// This is meant to be shared with anything that wishes to communicate with the editor
    sender: Sender<EditorMessage>,

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
    pub fn new(lsp_sender: Sender<LspControlerMessage>, lsp_listener: Rc<Receiver<LspControllerMessage>>) -> Self {
        terminal::enable_raw_mode().expect("Failed to enable raw mode");
        execute!(std::io::stdout(), terminal::EnterAlternateScreen).expect("Failed to enter alternate screen");
        execute!(std::io::stdout(), SetCursorStyle::BlinkingBlock).expect("Failed to set cursor style");


        let size = terminal::size()
            .map(|(w, h)| (w as usize, h as usize)).unwrap();

        let (sender, mailbox) = std::sync::mpsc::channel();

        let window = Window::new(sender.clone(), lsp_sender.clone(), lsp_listener.clone());


        let window_sender = window.get_sender();

        //todo: load settings from file
        let settings = Settings::default();

        let poll_duration = Duration::from_millis(settings.editor_settings.poll_timeout);



        let settings = Rc::new(RefCell::new(settings));

        Self {
            size,
            windows: vec![window],
            window_senders: vec![window_sender],
            active_window: 0,
            mailbox,
            sender,
            lsp_listener,
            lsp_sender,
            registers: Registers::new(),
            settings,
            compositor: Compositor::new(size),
            output_buffer: OutputBuffer::new(size),
        }

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

    pub fn style(&self) -> Option<StyledContent<String>> {
        if !self.changed {
            return None;
        }
        Some(apply_colors!(self.chr.to_string(), self.color))
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

    pub fn draw(&self, output: &mut OutputBuffer) {

        let rows = cmp::min(self.rows, self.contents.len());
        let cols = cmp::min(self.cols, self.contents[0].len());
        
        for y in 0..rows {
            for x in 0..cols {
                output.push(self.contents[y][x].style());
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



pub struct OutputRow {
    contents: Vec<String>,
    index: usize,
}

impl OutputRow {
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
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
    contents: Vec<OutputRow>,
    current_row: usize,
}


impl OutputBuffer {
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
            current_row: 0,
        }

    }

    pub fn clear(&mut self) {
        self.current_row = 0;
        for row in self.contents.iter_mut() {
            row.clear();
        }
    }

    pub fn push(&mut self, content: Option<StyledContent<String>>) {

        if self.current_row >= self.contents.len() {
            self.contents.push(OutputRow::new());
            if self.contents[self.current_row].push(content) {
                self.current_row += 1;
            }
        }
        else {
            if self.contents[self.current_row].push(content) {
                self.current_row += 1;
            }
        }
    }

    pub fn resize(&mut self, (cols, rows): (usize, usize)) {
        for row in self.contents.iter_mut() {
            row.resize(cols);
        }
        self.contents.truncate(rows);
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
        let mut out;
        for row in self.contents.iter() {
            out = row.write();

            if out.is_err() {
                return out;
            }
        }
        std::io::stdout().flush()?;
        self.clear();
        out
    }

}
