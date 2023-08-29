use std::cell::RefCell;
use std::cmp;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::io;
use std::io::Write;
use std::sync::mpsc::{Sender, Receiver, self};
use std::time::Duration;

use crossterm::event::{KeyEvent, self, Event};
use crossterm::style::{Stylize, StyledContent};
use crossterm::{terminal::{self, ClearType}, execute, cursor, queue};

use crate::mode::Mode;
use crate::{apply_colors, settings::Settings};
use crate::pane::{Pane, PaneContainer};
use crate::pane::text::TextPane;



pub enum Message {
    HorizontalSplit,
    VerticalSplit,
    ForceQuitAll,
    PaneUp,
    PaneDown,
    PaneLeft,
    PaneRight,
    OpenFile(String),
    ClosePane,
}

pub struct Window{
    size: (usize, usize),
    contents: WindowContents,
    active_pane: usize,
    panes: Vec<PaneContainer>,
    known_file_types: HashSet<String>,
    settings: Rc<RefCell<Settings>>,
    duration: Duration,
    channels: (Sender<Message>, Receiver<Message>),
    
}

impl Window {
    pub fn new() -> Self {
        let settings = Settings::default();
        let duration = Duration::from_millis(settings.editor_settings.key_timeout);

        let settings = Rc::new(RefCell::new(settings));
        

        let channels = mpsc::channel();
        
        let win_size = terminal::size()
            .map(|(w, h)| (w as usize, h as usize - 1))// -1 for trailing newline and -1 for command bar
            .unwrap();
        let pane: Rc<RefCell<dyn Pane>> = Rc::new(RefCell::new(TextPane::new(settings.clone(), channels.0.clone())));

        pane.borrow_mut().set_cursor_size(win_size);
        
        let panes = vec![PaneContainer::new(win_size, win_size, pane.clone(), settings.clone())];
        let mut known_file_types = HashSet::new();
        known_file_types.insert("txt".to_string());
        
        Self {
            size: win_size,
            contents: WindowContents::new(),
            active_pane: 0,
            panes,
            known_file_types,
            duration,
            settings,
            channels,
        }
    }

    fn open_file(&mut self, filename: PathBuf) -> usize {
        let file_type = filename.extension().and_then(|s| s.to_str()).unwrap_or("txt").to_string();

        let pane = match file_type.as_str() {
            "txt" | _ => {
                let mut pane = TextPane::new(self.settings.clone(), self.channels.0.clone());
                pane.open_file(&filename).expect("Failed to open file");
                pane
            }
        };
        let pane = Rc::new(RefCell::new(pane));
        self.panes.push(PaneContainer::new((0,0), (0, 0), pane.clone(), self.settings.clone()));
        self.panes.len() - 1
    }

    fn switch_pane(&mut self, filename: String) {
        let filename = PathBuf::from(filename);
        eprintln!("switching to pane: {:?}", filename);
        let mut pane_index = None;
        for (i, pane) in self.panes.iter().enumerate() {
            if pane.get_pane().borrow().get_filename() == &Some(filename.clone()) {//todo: remove the clone
                pane_index = Some(i);
                break;
            }
        }

        let new_active_pane_index = if let Some(pane_index) = pane_index {
            pane_index
        }
        else {
            self.open_file(filename)
        };

        let active_pane = self.panes[self.active_pane].get_pane().clone();
        let new_active_pane = self.panes[new_active_pane_index].get_pane().clone();

        self.panes[self.active_pane].change_pane(new_active_pane);
        self.panes[new_active_pane_index].change_pane(active_pane);

        self.panes[self.active_pane].get_pane().borrow_mut().set_cursor_size(self.panes[self.active_pane].get_size());
    }

    fn insert_pane(&mut self, index: usize, pane: PaneContainer) {
        let parent_pane = index - 1;
        self.panes.insert(index, pane);
    }

    fn remove_panes(&mut self) {
        let mut panes_to_remove = Vec::new();
        for (i, pane) in self.panes.iter().enumerate() {
            if pane.can_close() {
                panes_to_remove.push(i);
            }
        }

        
        for i in panes_to_remove.iter().rev() {
            
            loop {
                if *i + 1 < self.panes.len() {
                    let corners = self.panes[*i].get_corners();
                    if self.panes[*i + 1].combine(corners) {
                        break;
                    }
                }
                if *i != 0 {
                    let corners = self.panes[*i].get_corners();
                    if self.panes[*i - 1].combine(corners) {
                        break;
                    }
                }
                break;
            }

            
            self.panes.remove(*i);
        }
        if self.panes.len() == 0 {
            self.active_pane = 0;
        }
        else {
            self.active_pane = cmp::min(self.active_pane, self.panes.len() - 1);
        }

    }



    fn horizontal_split(&mut self) {
        //eprintln!("split panes: {:?}", self.panes.len());
        let active_pane_size = self.panes[self.active_pane].get_size();
        let new_pane_size = (active_pane_size.0, active_pane_size.1 / 2);
        let old_pane_size = if active_pane_size.1 % 2 == 0 {
            new_pane_size
        }
        else {
            (new_pane_size.0, new_pane_size.1 + 1)
        };

            
        self.panes[self.active_pane].set_size(old_pane_size);


        let new_pane_index = self.active_pane + 1;
        let mut new_pane = self.panes[self.active_pane].clone();

        let ((x,_), (_, y)) = self.panes[self.active_pane].get_corners();
        let new_pane_position = (x, y + 1);

        new_pane.set_position(new_pane_position);
        self.panes.insert(new_pane_index, new_pane);


        // This is for testing purposes, we need to make sure that we can actually access the new pane
        self.active_pane = new_pane_index;

        //eprintln!("split panes: {:?}", self.panes.len());
    }

    fn vertical_split(&mut self) {
        let active_pane_size = self.panes[self.active_pane].get_size();
        let new_pane_size = (active_pane_size.0 / 2, active_pane_size.1);
        let old_pane_size = if active_pane_size.0 % 2 == 0 {
            new_pane_size
        }
        else {
            (new_pane_size.0 + 1, new_pane_size.1)
        };
        self.panes[self.active_pane].set_size(old_pane_size);


        let new_pane_index = self.active_pane + 1;
        let mut new_pane = self.panes[self.active_pane].clone();

        let ((_,y), (x, _)) = self.panes[self.active_pane].get_corners();
        let new_pane_position = (x + 1, y);

        new_pane.set_position(new_pane_position);
        self.panes.insert(new_pane_index, new_pane);
        

        self.active_pane = new_pane_index;
    }


    fn pane_up(&mut self) {
        let ((x1, y1), (x2, _)) = self.panes[self.active_pane].get_corners();

        let pane_top = y1.saturating_sub(1);

        let pane_middle = (x1 + x2) / 2;

        let mut pane_index = None;
        // This loop tries to find the pane that is above the current pane
        for (i, pane) in self.panes.iter().enumerate() {
            let ((x1, _), (x2, y2)) = pane.get_corners();
            if y2 == pane_top && x1 <= pane_middle && pane_middle <= x2 {
                pane_index = Some(i);
                break;
            }
        }

        match pane_index {
            Some(index) => {
                self.active_pane = index;
            }
            None => {}
        }
    }

    fn pane_down(&mut self) {
        let ((x1, _), (x2, y2)) = self.panes[self.active_pane].get_corners();

        // We add 1 to make sure that we aren't on the current pane
        let pane_bottom = y2 + 1;
        let pane_middle = (x1 + x2) / 2;

        let mut pane_index = None;
        // This loop tries to find the pane that is below the current pane
        for (i, pane) in self.panes.iter().enumerate() {
            let ((x1, y1), (x2, _)) = pane.get_corners();
            if y1 == pane_bottom && x1 <= pane_middle && pane_middle <= x2 {
                pane_index = Some(i);
                break;
            }
        }

        match pane_index {
            Some(index) => {
                self.active_pane = index;
            }
            None => {}
        }
    }

    fn pane_right(&mut self) {
        let ((_, y1), (x2, y2)) = self.panes[self.active_pane].get_corners();

        // We add 1 to make sure that we aren't on the current pane
        let pane_right = x2 + 1;

        let pane_middle = (y1 + y2) / 2;

        let mut pane_index = None;
        // This loop tries to find the pane that is to the right of the current pane
        for (i, pane) in self.panes.iter().enumerate() {
            let ((x1, y1), (_, y2)) = pane.get_corners();
            if x1 == pane_right && y1 <= pane_middle && pane_middle <= y2 {
                pane_index = Some(i);
                break;
            }
        }

        match pane_index {
            Some(index) => {
                self.active_pane = index;
            }
            None => {}
        }
    }

    fn pane_left(&mut self) {
        let ((x1, y1), (_, y2)) = self.panes[self.active_pane].get_corners();

        let pane_left = x1.saturating_sub(1);

        let pane_middle = (y1 + y2) / 2;

        let mut pane_index = None;
        // This loop tries to find the pane that is to the left of the current pane
        for (i, pane) in self.panes.iter().enumerate() {
            let ((_, y1), (x2, y2)) = pane.get_corners();
            if x2 == pane_left && y1 <= pane_middle && pane_middle <= y2 {
                pane_index = Some(i);
                break;
            }
        }

        match pane_index {
            Some(index) => {
                self.active_pane = index;
            }
            None => {}
        }
    }

    fn read_messages(&mut self) {
        match self.channels.1.try_recv() {
            Ok(message) => {
                match message {
                    Message::HorizontalSplit => {
                        self.horizontal_split();
                    }
                    Message::VerticalSplit => {
                        self.vertical_split();
                    }
                    Message::ForceQuitAll => {
                        for pane in self.panes.iter_mut() {
                            pane.close();
                        }
                    }
                    Message::PaneUp => {
                        self.pane_up();
                    },
                    Message::PaneDown => {
                        self.pane_down();
                    },
                    Message::PaneLeft => {
                        self.pane_left();
                    },
                    Message::PaneRight => {
                        self.pane_right();
                    },
                    Message::OpenFile(path) => {
                        self.switch_pane(path);
                    }
                    Message::ClosePane => {
                        self.panes.remove(self.active_pane);
                        self.active_pane = self.active_pane.saturating_sub(1);
                    }
                }
            },
            Err(_) => {}
        }
    }

    fn process_event(&mut self) -> io::Result<Event> {
        loop {
            if event::poll(self.duration)? {
                return event::read();
            }
        }
    }


    pub fn run(&mut self) -> io::Result<bool> {
        //self.refresh_screen()?;
        self.read_messages();
        self.remove_panes();
        if self.panes.len() == 0 {
            return Ok(false);
        }
        self.refresh_screen()?;

        let ((x1, y1), (x2, y2)) = self.panes[self.active_pane].get_corners();

        if x1 == x2 || y1 == y2 {
            return Ok(false);
        }
        
        let event = self.process_event()?;
        match event {
            Event::Key(key) => {
                self.contents.set_change(true);
                self.process_keypress(key)
            },
            Event::Resize(width, height) => {
                self.contents.set_change(true);
                self.resize(width, height);
                Ok(true)
            }
            _ => {
                self.contents.set_change(false);
                Ok(true)},
        }
    }

    fn resize(&mut self, width: u16, height: u16) {
        self.size = (width as usize, height as usize - 1);
        for pane in self.panes.iter_mut() {
            pane.resize((width as usize, height as usize));
        }
    }

    pub fn clear_screen() -> io::Result<()> {
        execute!(std::io::stdout(), terminal::Clear(terminal::ClearType::All))?;
        execute!(std::io::stdout(), cursor::MoveTo(0, 0))
    }


    
    fn draw_rows(&mut self) {
        let rows = self.size.1;
        //let cols = self.size.0;


        //eprintln!("panes: {}", self.panes.len());
        //let panes = self.panes.len();

        let offset = 0;
        for i in 0..rows {

            let mut pane_index = 0;
            let mut window_index = 0;
            //eprintln!("size: {:?} i: {}", self.size, i);
            while window_index < self.size.0 {
                //eprintln!("window_index: {} pane_index: {}\r\n", window_index, pane_index);
                if pane_index >= self.panes.len() {
                    break;
                }
                //eprintln!("pane size: {:?} pane_index: {}", self.panes[pane_index].borrow().size, pane_index);
                //eprintln!("pane corners: {:?}", self.panes[pane_index].borrow().get_corners());
                let ((start_x, start_y), (end_x, end_y)) = self.panes[pane_index].get_corners();
                if start_y <= i && end_y >= i {
                    self.panes[pane_index].draw_row(i - start_y + offset, &mut self.contents);
                    window_index += end_x - start_x + 1;
                    /*if window_index < self.size.0 {
                        self.contents.push_str("|");
                        window_index += 1;
                    }
                    row_drawn = true;*/
                }
                pane_index += 1;
            }


            //self.contents.merge(&mut self.panes[self.active_pane].borrow().draw_row(i));
            /*if !row_drawn {
                self.contents.push_str("-".repeat(cols).as_str());
                offset = 1;
            }*/
            
            /*queue!(
                self.contents,
                terminal::Clear(ClearType::UntilNewLine),
            ).unwrap();*/

            let color_settings = &self.settings.borrow().colors.pane;

            self.contents.push_str(apply_colors!("\r\n", color_settings));


        }

    }


    pub fn draw_status_bar(&mut self) {
        Self::clear_screen().unwrap();
        queue!(
            self.contents,
            terminal::Clear(ClearType::UntilNewLine),
        ).unwrap();

        let settings = self.settings.borrow();
        
        let color_settings = &settings.colors.ui;

        let (name, first, second) = self.panes[self.active_pane].get_status();
        let total = name.len() + 1 + first.len() + second.len();// plus one for the space

        let mode_color = &settings.colors.mode.get(&name).unwrap_or(&color_settings);

        self.contents.push_str(apply_colors!(format!("{}", name), mode_color));

        self.contents.push_str(apply_colors!(" ", color_settings));


        self.contents.push_str(apply_colors!(first, color_settings));
        
        let remaining = self.size.0.saturating_sub(total);

        self.contents.push_str(apply_colors!(" ".repeat(remaining), color_settings));


        self.contents.push_str(apply_colors!(second, color_settings));
    }

    pub fn refresh_screen(&mut self) -> io::Result<()> {

        if !self.contents.will_change() {
            return Ok(());
        }
        
        self.panes[self.active_pane].refresh();

        self.panes[self.active_pane].scroll_cursor();

        queue!(
            self.contents,
            cursor::Hide,
            cursor::MoveTo(0, 0),
        )?;

        self.draw_rows();
        self.draw_status_bar();

        let cursor = self.panes[self.active_pane].get_cursor();
        let cursor = cursor.borrow();

        let (x, y) = cursor.get_real_cursor();
        //eprintln!("x: {} y: {}", x, y);
        let x = x + self.panes[self.active_pane].get_position().0;
        let y = y + self.panes[self.active_pane].get_position().1;
        //eprintln!("x: {} y: {}", x, y);

        
        let x = x + cursor.number_line_size;

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
        self.panes[self.active_pane].open_file(&PathBuf::from(filename.to_owned()))
    }

    pub fn process_keypress(&mut self, key: KeyEvent) -> io::Result<bool> {
        self.panes[self.active_pane].process_keypress(key)
    }

}

pub trait WindowContentsUtils<T> {
    fn push_str(&mut self, s: T);
}

pub struct WindowContents {
    content: String,
    change: bool,
}

impl WindowContents {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            change: true,
        }
    }

    fn push(&mut self, c: char) {
        self.content.push(c);
        self.change = true;
    }

    fn merge(&mut self, other: &mut Self) {
        self.content.push_str(other.content.as_str());
        other.content.clear();
        self.change = true;
    }

    fn set_change(&mut self, change: bool) {
        self.change = change;
    }

    fn will_change(&self) -> bool {
        self.change
    }
}

impl WindowContentsUtils<&str> for WindowContents {
    fn push_str(&mut self, s: &str) {
        self.content.push_str(s);
        self.change = true;
    }
}

impl WindowContentsUtils<String> for WindowContents {
    fn push_str(&mut self, s: String) {
        self.content.push_str(s.as_str());
        self.change = true;
    }
}

impl WindowContentsUtils<&String> for WindowContents {
    fn push_str(&mut self, s: &String) {
        self.content.push_str(s.as_str());
        self.change = true;
    }
}

impl WindowContentsUtils<StyledContent<&str>> for WindowContents {
    fn push_str(&mut self, s: StyledContent<&str>) {
        self.content.push_str(&format!("{}", s));
        self.change = true;
    }
}

impl WindowContentsUtils<StyledContent<String>> for WindowContents {
    fn push_str(&mut self, s: StyledContent<String>) {
        self.content.push_str(&format!("{}", s));
        self.change = true;
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
        if !self.change {
            return Ok(());
        }
        let out = write!(std::io::stdout(), "{}", self.content);
        std::io::stdout().flush()?;
        self.content.clear();
        self.change = true;
        out
    }
}


