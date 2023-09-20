use std::{rc::Rc, cell::RefCell, sync::mpsc::{Sender, Receiver}, path::PathBuf, io};


use uuid::Uuid;

use crate::{mode::{Mode, PromptType, Promptable}, cursor::Cursor, window::{StyledChar, WindowMessage, TextRow}, settings::{Settings, Key}, buffer::Buffer};
use super::{PaneMessage, PaneContainer, Pane};








pub struct PopUpPane {
    mode : Rc<RefCell<dyn Promptable>>,
    window_sender: Sender<WindowMessage>,
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
                      window_sender: Sender<WindowMessage>,
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
                    window_sender: Sender<WindowMessage>,
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

    pub fn new_dropdown(settings: Rc<RefCell<Settings>>,
                        prompt: Vec<String>,
                        window_sender: Sender<WindowMessage>,
                        pane_sender: Sender<PaneMessage>,
                        pane_receiver: Receiver<PaneMessage>,
                        buttons: PromptType,
                        border: bool) -> PopUpPane {

        let mode = Rc::new(RefCell::new(crate::mode::drop_down::DropDown::new(buttons)));

        mode.borrow_mut().add_keybindings(settings.borrow().mode_keybindings.get("Drop Down").unwrap().clone());

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


    fn check_messages(&mut self, container: &mut PaneContainer) {
        match self.pane_receiver.try_recv() {
            Ok(message) => {
                match message {
                    PaneMessage::String(_string) => {
                    },
                    PaneMessage::Close => self.run_command(&format!("close {}", container.get_uuid()), container),
                }
            },
            Err(_) => {},
        }
    }
    

}


impl Pane for PopUpPane {

    fn changed(&mut self) {}
    
    fn reset(&mut self) {
    }
    

    fn refresh(&mut self, container: &mut PaneContainer) {
        self.check_messages(container);
    }

    fn change_mode(&mut self, name: &str) {}

    fn process_keypress(&mut self, key: Key, container: &mut PaneContainer) {
        let mode = self.mode.clone();
        let result = mode.borrow_mut().process_keypress(key, self, container);
        result
    }

    fn draw_row(&self, index: usize, container: &PaneContainer, output: &mut TextRow) {

        let (width, height) = container.get_size();
        
        let color_settings = container.settings.borrow().colors.clone().popup.clone();

        if self.border {
            if index == 0 {
                output.push(Some(Some(StyledChar::new('┌', color_settings.clone()))));
                for _ in 0..width - 2 {
                    output.push(Some(Some(StyledChar::new('─', color_settings.clone()))));
                }
                output.push(Some(Some(StyledChar::new('┐', color_settings.clone()))));

                *self.prompt_level.borrow_mut() = 0;
                *self.drawn_prompt.borrow_mut() = 0;
            }
            else if index == height {
                output.push(Some(Some(StyledChar::new('└', color_settings.clone()))));
                for _ in 0..width - 2 {
                    output.push(Some(Some(StyledChar::new('─', color_settings.clone()))));
                }
                output.push(Some(Some(StyledChar::new('┘', color_settings.clone()))));
            }
            else {
                output.push(Some(Some(StyledChar::new('│', color_settings.clone()))));

                if *self.drawn_prompt.borrow() < self.prompt.len() {
                    let prompt = *self.drawn_prompt.borrow();
                    let side_len = width.saturating_sub(2 + self.prompt[prompt].chars().count());
                    let side_len = side_len / 2;
                    for _ in 0..side_len {
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }

                    for c in self.prompt[prompt].chars() {
                        output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                    }

                    for _ in 0..side_len {
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }
                    if side_len * 2 + self.prompt[prompt].chars().count() + 2 < width {
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }

                    *self.drawn_prompt.borrow_mut() += 1;
                }
                else if *self.drawn_prompt.borrow() == self.prompt.len() && self.prompt.len() > 0 {
                    for _ in 0..width - 2 {
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }
                    *self.drawn_prompt.borrow_mut() += 1;
                }
                else {
                    let row_offset = *self.prompt_level.borrow();
                    let mode = self.mode.clone();
                    
                    let row_output = mode.borrow_mut().draw_prompt(index - index + row_offset, container);

                    let len = row_output.len();

                    let side_len = width.saturating_sub(2 + len);
                    let side_len = side_len / 2;
                    for _ in 0..side_len {
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }

                    output.extend(row_output);

                    for _ in 0..side_len {
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }
                    if side_len * 2 + len < (width - 2) {
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }

                    *self.prompt_level.borrow_mut() += 1;
                }
                output.push(Some(Some(StyledChar::new('│', color_settings.clone()))));
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
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }

                    for c in self.prompt[prompt].chars() {
                        output.push(Some(Some(StyledChar::new(c, color_settings.clone()))));
                    }

                    for _ in 0..side_len {
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }

                    if side_len * 2 + self.prompt[prompt].chars().count() < width {
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }

                    *self.drawn_prompt.borrow_mut() += 1;
                }
                else if *self.drawn_prompt.borrow() == self.prompt.len() && self.prompt.len() > 0 {
                    for _ in 0..width {
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }
                    *self.drawn_prompt.borrow_mut() += 1;
                }
                else {
                    let row_offset = *self.prompt_level.borrow();
                    let mode = self.mode.clone();
                    
                    let row_output = mode.borrow_mut().draw_prompt(index - index + row_offset, container);

                    let len = row_output.len();

                    let side_len = width.saturating_sub(len);
                    let side_len = side_len / 2;
                    for _ in 0..side_len {
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }

                    output.extend(row_output);

                    for _ in 0..side_len {
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }
                    if side_len * 2 + len < width {
                        output.push(Some(Some(StyledChar::new(' ', color_settings.clone()))));
                    }

                    *self.prompt_level.borrow_mut() += 1;
                }
            }
        }
    }

    fn run_command(&mut self, command: &str, _container: &mut PaneContainer) {
        let mut command_args = command.split(" ");

        let command = command_args.next().unwrap();
        
        match command {
            "cancel" => {
                self.window_sender.send(WindowMessage::ClosePane(true, None)).unwrap();
            },
            "submit" => {
                let result_type = command_args.next().unwrap();

                match result_type {
                    "text" => {
                        let value = command_args.next().unwrap();
                        self.window_sender.send(WindowMessage::ClosePane(true, None)).unwrap();
                        self.pane_sender.send(PaneMessage::String(value.to_string())).unwrap();
                    },
                    "radio" => {
                        let value = command_args.next().unwrap();
                        self.window_sender.send(WindowMessage::ClosePane(true, None)).unwrap();
                        self.pane_sender.send(PaneMessage::String(value.to_string())).unwrap();
                    },
                    "button" => {
                        let value = command_args.collect::<Vec<&str>>().join(" ");
                        self.window_sender.send(WindowMessage::ClosePane(true, None)).unwrap();
                        match self.pane_sender.send(PaneMessage::String(value.to_string())) {
                            Ok(_) => {},
                            Err(e) => {
                                eprintln!("Error sending message: {}", e);
                            }
                        };
                    },
                    "checkbox" => {
                        let value = command_args.next().unwrap();
                        self.window_sender.send(WindowMessage::ClosePane(true, None)).unwrap();
                        self.pane_sender.send(PaneMessage::String(value.to_string())).unwrap();
                    },
                    x => {
                        panic!("Unknown result type {}", x);
                    }
                }
            },
            "close" => {
                if let Some(value) = command_args.next() {
                    self.window_sender.send(WindowMessage::ClosePane(true, Some(Uuid::try_parse(value).unwrap()))).unwrap();
                }
                else {
                    self.window_sender.send(WindowMessage::ClosePane(true, None)).unwrap();
                }
            },
            _x => {}
        }
    }
        


    fn get_status(&self, _container: &PaneContainer) -> Option<(String, String, String)> {
        None
    }


    fn get_cursor(&self) -> Option<(usize, usize)> {
        None
    }

    fn get_name(&self) -> &str {
        "Prompt"
    }

    
    fn get_settings(&self) -> Rc<RefCell<Settings>> {
        self.settings.clone()
    }



    fn resize(&mut self, _size: (usize, usize)) {
    }

    fn set_location(&mut self, _location: (usize, usize)) {

    }

    fn backup(&mut self) {

    }

}
