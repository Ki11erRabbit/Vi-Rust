use std::{rc::Rc, cell::RefCell, sync::mpsc::Sender, path::PathBuf, io};


use crate::{mode::prompt::Promptable, cursor::Cursor, window::{OutputSegment, Message}};
use super::{PaneMessage, PaneContainer, Pane};








pub struct PopUpPane {
    mode : Rc<RefCell<dyn Promptable>>,
    window_sender: Sender<Message>,
    pane_sender: Sender<PaneMessage>,
    prompt: String,
    input: String,
}


impl Pane for PopUpPane {
    fn scroll_cursor(&mut self, container: &PaneContainer) {

    }

    fn refresh(&mut self) {
    }

    fn change_mode(&mut self, name: &str) {}

    fn process_keypress(&mut self, key: crossterm::event::KeyEvent) -> io::Result<bool> {

        Ok(false)
    }

    fn draw_row(&self, index: usize, container: &PaneContainer) -> Vec<OutputSegment> {
        let mut output = Vec::new();

        let color_settings = container.settings.borrow().colors.clone().ui;
        
        if index == 0 {
            

        }

        output
    }

    fn run_command(&mut self, command: &str) {
        let mut command_args = command.split(" ");

        let command = command_args.next().unwrap();
        
        match command {
            "cancel" => {
                self.window_sender.send(Message::ClosePane).unwrap();
            },
            "submit" => {
                let result_type = command_args.next().unwrap();

                match result_type {
                    "text" => {
                        let value = command_args.next().unwrap();
                        self.window_sender.send(Message::ClosePane).unwrap();
                        self.pane_sender.send(PaneMessage::String(value.to_string())).unwrap();
                    },
                    "radio" => {
                        let value = command_args.next().unwrap();
                        self.window_sender.send(Message::ClosePane).unwrap();
                        self.pane_sender.send(PaneMessage::String(value.to_string())).unwrap();
                    },
                    "button" => {
                        let value = command_args.next().unwrap();
                        self.window_sender.send(Message::ClosePane).unwrap();
                        self.pane_sender.send(PaneMessage::String(value.to_string())).unwrap();
                    },
                    "checkbox" => {
                        let value = command_args.next().unwrap();
                        self.window_sender.send(Message::ClosePane).unwrap();
                        self.pane_sender.send(PaneMessage::String(value.to_string())).unwrap();
                    },
                    x => {
                        panic!("Unknown result type {}", x);
                    }
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
        

    

}
