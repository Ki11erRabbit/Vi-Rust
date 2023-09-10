use std::{rc::Rc, cell::RefCell, sync::mpsc::{Sender, Receiver}, path::PathBuf, io};


use uuid::Uuid;

use crate::{mode::{Mode, prompt::PromptType, Promptable}, cursor::Cursor, window::{StyledChar, Message}, settings::Settings, buffer::Buffer};
use super::{PaneMessage, PaneContainer, Pane};








pub struct PopUpPane {
    mode : Rc<RefCell<dyn Promptable>>,
    window_sender: Sender<Message>,
    pane_sender: Sender<PaneMessage>,
    pane_receiver: Receiver<PaneMessage>,
    prompt: Vec<String>,
    drawn_prompt: RefCell<usize>,
    prompt_level: RefCell<usize>,
    settings: Rc<RefCell<Settings>>,
    border: bool,
}

impl PopUpPane {
    pub fn new_prompt(settings: Rc<RefCell<Settings>>,
                      prompt: Vec<String>,
                      window_sender: Sender<Message>,
                      pane_sender: Sender<PaneMessage>,
                      pane_receiver: Receiver<PaneMessage>,
                      prompts: Vec<PromptType>,
                      border: bool) -> PopUpPane {

        let mode = Rc::new(RefCell::new(crate::mode::prompt::Prompt::new(prompts)));

        mode.borrow_mut().add_keybindings(settings.borrow().mode_keybindings.get("Prompt").unwrap().clone());
        
        PopUpPane {
            mode,
            window_sender,
            pane_sender,
            pane_receiver,
            prompt,
            drawn_prompt: RefCell::new(0),
            prompt_level: RefCell::new(0),
            settings,
            border
        }
    }

    pub fn new_info(settings: Rc<RefCell<Settings>>,
                    prompt: Vec<String>,
                    window_sender: Sender<Message>,
                    pane_sender: Sender<PaneMessage>,
                    pane_receiver: Receiver<PaneMessage>,
                    body: Vec<Option<String>>,
                    border: bool) -> PopUpPane {

        let mode = Rc::new(RefCell::new(crate::mode::info::Info::new(body)));

        //mode.borrow_mut().add_keybindings(settings.borrow().mode_keybindings.get("Info").unwrap().clone());
        
        PopUpPane {
            mode,
            window_sender,
            pane_sender,
            pane_receiver,
            prompt,
            drawn_prompt: RefCell::new(0),
            prompt_level: RefCell::new(0),
            settings,
            border
        }
    }


    fn check_messages(&mut self, container: &PaneContainer) {
        match self.pane_receiver.try_recv() {
            Ok(message) => {
                match message {
                    PaneMessage::String(string) => {
                    },
                    PaneMessage::Close => self.run_command(&format!("close {}", container.get_uuid()), container),
                }
            },
            Err(_) => {},
        }
    }
    

}


impl Pane for PopUpPane {
    fn scroll_cursor(&mut self, container: &PaneContainer) {

    }

    fn refresh(&mut self, container: &mut PaneContainer) {
        self.check_messages(container);
    }

    fn change_mode(&mut self, name: &str) {}

    fn process_keypress(&mut self, key: crossterm::event::KeyEvent, container: &mut PaneContainer) -> io::Result<bool> {
        let mode = self.mode.clone();
        let result = mode.borrow_mut().process_keypress(key, self, container);
        result
    }

    fn draw_row(&self, index: usize, container: &PaneContainer, output: &mut Vec<Option<StyledChar>>){

        let (width, height) = container.get_size();
        
        let color_settings = container.settings.borrow().colors.clone().popup.clone();

        if self.border {
            if index == 0 {
                output.push(Some(StyledChar::new('┌', color_settings.clone())));
                for _ in 0..width - 2 {
                    output.push(Some(StyledChar::new('─', color_settings.clone())));
                }
                output.push(Some(StyledChar::new('┐', color_settings.clone())));

                *self.prompt_level.borrow_mut() = 0;
                *self.drawn_prompt.borrow_mut() = 0;
            }
            else if index == height {
                output.push(Some(StyledChar::new('└', color_settings.clone())));
                for _ in 0..width - 2 {
                    output.push(Some(StyledChar::new('─', color_settings.clone())));
                }
                output.push(Some(StyledChar::new('┘', color_settings.clone())));
            }
            else {
                output.push(Some(StyledChar::new('│', color_settings.clone())));

                if *self.drawn_prompt.borrow() < self.prompt.len() {
                    let prompt = *self.drawn_prompt.borrow();
                    let side_len = width.saturating_sub(2 + self.prompt[prompt].chars().count());
                    let side_len = side_len / 2;
                    for _ in 0..side_len {
                        output.push(Some(StyledChar::new(' ', color_settings.clone())));
                    }

                    for c in self.prompt[prompt].chars() {
                        output.push(Some(StyledChar::new(c, color_settings.clone())));
                    }

                    for _ in 0..side_len {
                        output.push(Some(StyledChar::new(' ', color_settings.clone())));
                    }

                    *self.drawn_prompt.borrow_mut() += 1;
                }
                else if *self.drawn_prompt.borrow() == self.prompt.len() {
                    for _ in 0..width - 2 {
                        output.push(Some(StyledChar::new(' ', color_settings.clone())));
                    }
                    *self.drawn_prompt.borrow_mut() += 1;
                }
                else {
                    let row_offset = *self.prompt_level.borrow();
                    let mode = self.mode.clone();
                    mode.borrow_mut().draw_prompt(index - index + row_offset, container, output);

                    *self.prompt_level.borrow_mut() += 1;
                }
                output.push(Some(StyledChar::new('│', color_settings.clone())));
            }
        }
        else {
            if index == 0 {
                *self.prompt_level.borrow_mut() = 0;
                *self.drawn_prompt.borrow_mut() = 0;
            }
            else if index == height {
                *self.prompt_level.borrow_mut() = 0;
                *self.drawn_prompt.borrow_mut() = 0;
            }
            else {
                if *self.drawn_prompt.borrow() < self.prompt.len() {
                    let prompt = *self.drawn_prompt.borrow();
                    let side_len = width.saturating_sub(self.prompt[prompt].chars().count());
                    let side_len = side_len / 2;
                    for _ in 0..side_len {
                        output.push(Some(StyledChar::new(' ', color_settings.clone())));
                    }

                    for c in self.prompt[prompt].chars() {
                        output.push(Some(StyledChar::new(c, color_settings.clone())));
                    }

                    for _ in 0..side_len {
                        output.push(Some(StyledChar::new(' ', color_settings.clone())));
                    }

                    if side_len * 2 + self.prompt[prompt].chars().count() < width {
                        output.push(Some(StyledChar::new(' ', color_settings.clone())));
                    }

                    *self.drawn_prompt.borrow_mut() += 1;
                }
                else if *self.drawn_prompt.borrow() == self.prompt.len() {
                    for _ in 0..width {
                        output.push(Some(StyledChar::new(' ', color_settings.clone())));
                    }
                    *self.drawn_prompt.borrow_mut() += 1;
                }
                else {
                    let row_offset = *self.prompt_level.borrow();
                    let mode = self.mode.clone();
                    mode.borrow_mut().draw_prompt(index - index + row_offset, container, output);

                    *self.prompt_level.borrow_mut() += 1;
                }
            }
        }
    }

    fn run_command(&mut self, command: &str, container: &PaneContainer) {
        let mut command_args = command.split(" ");

        let command = command_args.next().unwrap();
        
        match command {
            "cancel" => {
                self.window_sender.send(Message::ClosePane(true, None)).unwrap();
            },
            "submit" => {
                let result_type = command_args.next().unwrap();

                match result_type {
                    "text" => {
                        let value = command_args.next().unwrap();
                        self.window_sender.send(Message::ClosePane(true, None)).unwrap();
                        self.pane_sender.send(PaneMessage::String(value.to_string())).unwrap();
                    },
                    "radio" => {
                        let value = command_args.next().unwrap();
                        self.window_sender.send(Message::ClosePane(true, None)).unwrap();
                        self.pane_sender.send(PaneMessage::String(value.to_string())).unwrap();
                    },
                    "button" => {
                        let value = command_args.next().unwrap();
                        self.window_sender.send(Message::ClosePane(true, None)).unwrap();
                        self.pane_sender.send(PaneMessage::String(value.to_string())).unwrap();
                    },
                    "checkbox" => {
                        let value = command_args.next().unwrap();
                        self.window_sender.send(Message::ClosePane(true, None)).unwrap();
                        self.pane_sender.send(PaneMessage::String(value.to_string())).unwrap();
                    },
                    x => {
                        panic!("Unknown result type {}", x);
                    }
                }
            },
            "close" => {
                if let Some(value) = command_args.next() {
                    self.window_sender.send(Message::ClosePane(true, Some(Uuid::try_parse(value).unwrap()))).unwrap();
                }
                else {
                    self.window_sender.send(Message::ClosePane(true, None)).unwrap();
                }
            },
            x => {}
        }
    }
        

    fn save_buffer(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn open_file(&mut self, filename: &PathBuf) -> io::Result<()> {
        Ok(())
    }

    fn get_status(&self, container: &PaneContainer) -> (String, String, String) {
        ("".to_string(), "".to_string(), "".to_string())
    }

    fn insert_newline(&mut self) {}

    fn delete_char(&mut self) {}

    fn backspace_char(&mut self) {}

    fn insert_char(&mut self, c: char) {}

    fn insert_str(&mut self, s: &str) {}

    fn get_cursor(&self) -> Rc<RefCell<Cursor>> {
        panic!("Cannot get cursor from popup pane")
    }

    fn get_line_count(&self) -> usize {
        0
    }

    fn buffer_to_string(&self) -> String {
        "".to_string()
    }

    fn get_row_len(&self, row: usize) -> Option<usize> {
        None
    }

    fn get_filename(&self) -> &Option<std::path::PathBuf> {
        &None
    }

    fn resize_cursor(&mut self, size: (usize, usize)) {}

    fn set_cursor_size(&mut self, size: (usize, usize)) {}
        

    fn backup_buffer(&mut self) {
    }
    
    fn get_settings(&self) -> Rc<RefCell<Settings>> {
        self.settings.clone()
    }


    fn borrow_buffer(&self) -> &Buffer {
        unimplemented!()
    }
    fn borrow_mut_buffer(&mut self) -> &mut Buffer {
        unimplemented!()
    }


    fn set_sender(&mut self, sender: Sender<Message>) {
        unimplemented!()
    }

}
