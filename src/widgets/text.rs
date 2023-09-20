use std::{cell::RefCell, rc::Rc};

use crossterm::event::{KeyCode, KeyModifiers};

use crate::{settings::{Key, Settings, ColorScheme}, window::StyledChar};

use super::{Widget, InteractableWidget};






pub struct TextBox {
    /// The text to display
    text: String,
    /// Whether to hide the text or not (for passwords)
    hide: bool,
    /// The color scheme to use
    color_settings: ColorScheme,
}

impl TextBox {
    pub fn new(text: String, hide: bool) -> Self {
        Self { text,
               hide,
               color_settings: ColorScheme::ui()
        }
    }

    pub fn text_len(&self) -> usize {
        self.text.chars().count()
    }

    pub fn add_char(&mut self, c: char) {
        self.text.push(c);
    }

    pub fn remove_char(&mut self, index: usize) {
        self.text = self.text.chars()
            .enumerate()
            .filter(|(i, _)| *i == index)
            .map(|(_, c)| c)
            .collect();
    }

    pub fn add_str<S>(&mut self, str: S) where S: AsRef<str> {
        self.text.push_str(str.as_ref());
    }

    pub fn get_text(&self) -> String {
        self.text.clone()
    }

}

impl Widget for TextBox {
    fn draw(&self, _: usize, output: &mut Vec<Option<StyledChar>>) -> bool {

        
        if self.hide {
            //"*".repeat(self.text.chars().count())
            *output = "*".repeat(self.text.chars().count())
                .chars()
                .map(|c| Some(StyledChar::new(c, self.color_settings.clone())))
                .collect::<Vec<Option<StyledChar>>>();
        } else {
            *output = self.text.chars()
                .map(|c| Some(StyledChar::new(c, self.color_settings.clone())))
                .collect();
        }
        true
    }


    fn set_color_scheme(&mut self, color_scheme: ColorScheme) {
        self.color_settings = color_scheme;
    }
}



pub struct Label {
    /// The text to display
    text: TextBox,
}

impl Label {
    pub fn new(text: String) -> Self {
        Self { text: TextBox::new(text, false),
        }
    }
}

impl From<&str> for Label {
    fn from(text: &str) -> Self {
        Self::new(text.to_string())
    }
}

impl From<String> for Label {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

impl From<&String> for Label {
    fn from(text: &String) -> Self {
        Self::new(text.to_string())
    }
}

impl Widget for Label {
    fn draw(&self, index: usize, output: &mut Vec<Option<StyledChar>>) -> bool {
        self.text.draw(index, output)
    }

    fn set_color_scheme(&mut self, scheme: ColorScheme) {
        self.text.set_color_scheme(scheme);
    }
}

pub struct TextEntry {
    /// The text to display
    text: TextBox,
    /// The index of the cursor
    index: usize,
    /// Callback for when the widget is clicked
    on_click: Option<Rc<RefCell<dyn FnMut(&mut Self)>>>,
    /// Callback for when the widget is focused
    on_focus: Option<Rc<RefCell<dyn FnMut(&mut Self)>>>,
    /// Callback for when a key is pressed
    on_keypress: Option<Rc<RefCell<dyn FnMut(&mut Self, crate::settings::Key)>>>,
}


impl TextEntry {
    pub fn new(text: String) -> Self {
        Self { text: TextBox::new(text.clone(), false), index: text.chars().count() - 1, on_click: None, on_focus: None, on_keypress: None }
    }
}

impl Widget for TextEntry {
    fn draw(&self, index: usize, output: &mut Vec<Option<StyledChar>>) -> bool {
        self.text.draw(index, output)
    }


    fn set_color_scheme(&mut self, scheme: ColorScheme) {
        self.text.set_color_scheme(scheme);
    }
}

impl InteractableWidget for TextEntry {
    fn set_on_click<F>(&mut self, callback: F) where F: FnMut(&mut Self) + 'static {
        self.on_click = Some(Rc::new(RefCell::new(callback)));
    }

    fn set_on_focus<F>(&mut self, callback: F) where F: FnMut(&mut Self) + 'static {
        self.on_focus = Some(Rc::new(RefCell::new(callback)));
    }

    fn set_on_keypress<F>(&mut self, callback: F) where F: FnMut(&mut Self, crate::settings::Key) + 'static {
        self.on_keypress = Some(Rc::new(RefCell::new(callback)));
    }

    fn click(&mut self) {
        let mut callback = self.on_click.take();
        if let Some(ref mut callback) = callback {
            let call = callback.clone();
            call.borrow_mut()(self);
            self.on_click = Some(call);
        }
    }

    fn focus(&mut self) {
        let mut callback = self.on_focus.take();
        if let Some(ref mut callback) = callback {
            let call = callback.clone();
            call.borrow_mut()(self);
            self.on_focus = Some(call);
        }
    }

    fn keypress(&mut self, key: crate::settings::Key) {
        match &key {
            Key {
                key: code @ KeyCode::Char(..),
                modifier: KeyModifiers::NONE,
            } => {
                match code {
                    KeyCode::Char(c) => {
                        self.text.add_char(*c);
                        self.index += 1;
                    },
                    _ => unreachable!(),
                }
            },
            Key {
                key: KeyCode::Backspace,
                modifier: KeyModifiers::NONE,
            } => {
                self.text.remove_char(self.index);
                self.index -= 1;
            },
            Key {
                key: KeyCode::Delete,
                modifier: KeyModifiers::NONE,
            } => {
                self.text.remove_char(self.index + 1);
            },
            Key {
                key: KeyCode::Left,
                modifier: KeyModifiers::NONE,
            } => {
                self.index = self.index.saturating_sub(1);
            },
            Key {
                key: KeyCode::Right,
                modifier: KeyModifiers::NONE,
            } => {
                let temp = (1 + self.index) % self.text.text_len();
                if self.index < temp {
                    self.index = temp;
                }
            },
            Key {
                key: KeyCode::Up,
                modifier: KeyModifiers::NONE,
            } => {
                self.index = 0;
            },
            Key {
                key: KeyCode::Down,
                modifier: KeyModifiers::NONE,
            } => {
                self.index = self.text.text_len() - 1;
            },
            _ => {},
        }

        let mut callback = self.on_keypress.take();
        
        if let Some(ref mut callback) = callback {
            let call = callback.clone();
            call.borrow_mut()(self, key);
            self.on_keypress = Some(call);
        }

        
    }

}


pub struct PasswordEntry {
    /// The text to display
    text: TextBox,
    /// The index of the cursor
    index: usize,
    /// Callback for when the widget is clicked
    on_click: Option<Rc<RefCell<dyn FnMut(&mut Self)>>>,
    /// Callback for when the widget is focused
    on_focus: Option<Rc<RefCell<dyn FnMut(&mut Self)>>>,
    /// Callback for when a key is pressed
    on_keypress: Option<Rc<RefCell<dyn FnMut(&mut Self, crate::settings::Key)>>>,
}

impl PasswordEntry {
    pub fn new() -> Self {
        Self { text: TextBox::new(String::new(), true), index: 0, on_click: None, on_focus: None, on_keypress: None }
    }
}


impl Widget for PasswordEntry {
    fn draw(&self, index: usize, output: &mut Vec<Option<StyledChar>>) -> bool {
        self.text.draw(index, output)
    }


    fn set_color_scheme(&mut self, scheme: ColorScheme) {
        self.text.set_color_scheme(scheme);
    }
}

impl InteractableWidget for PasswordEntry {
    fn set_on_click<F>(&mut self, callback: F) where F: FnMut(&mut Self) + 'static {
        self.on_click = Some(Rc::new(RefCell::new(callback)));
    }

    fn set_on_focus<F>(&mut self, callback: F) where F: FnMut(&mut Self) + 'static {
        self.on_focus = Some(Rc::new(RefCell::new(callback)));
    }

    fn set_on_keypress<F>(&mut self, callback: F) where F: FnMut(&mut Self, crate::settings::Key) + 'static {
        self.on_keypress = Some(Rc::new(RefCell::new(callback)));
    }

    fn click(&mut self) {
        let mut callback = self.on_click.take();
        if let Some(ref mut callback) = callback {
            let call = callback.clone();
            call.borrow_mut()(self);
            self.on_click = Some(call);
        }
    }

    fn focus(&mut self) {
        let mut callback = self.on_focus.take();
        if let Some(ref mut callback) = callback {
            let call = callback.clone();
            call.borrow_mut()(self);
            self.on_focus = Some(call);
        }
    }

    fn keypress(&mut self, key: crate::settings::Key) {
        match &key {
            Key {
                key: code @ KeyCode::Char(..),
                modifier: KeyModifiers::NONE,
            } => {
                match code {
                    KeyCode::Char(c) => {
                        self.text.add_char(*c);
                        self.index += 1;
                    },
                    _ => unreachable!(),
                }
            },
            Key {
                key: KeyCode::Backspace,
                modifier: KeyModifiers::NONE,
            } => {
                self.text.remove_char(self.index);
                self.index -= 1;
            },
            Key {
                key: KeyCode::Delete,
                modifier: KeyModifiers::NONE,
            } => {
                self.text.remove_char(self.index + 1);
            },
            Key {
                key: KeyCode::Left,
                modifier: KeyModifiers::NONE,
            } => {
                self.index = self.index.saturating_sub(1);
            },
            Key {
                key: KeyCode::Right,
                modifier: KeyModifiers::NONE,
            } => {
                let temp = (1 + self.index) % self.text.text_len();
                if self.index < temp {
                    self.index = temp;
                }
            },
            Key {
                key: KeyCode::Up,
                modifier: KeyModifiers::NONE,
            } => {
                self.index = 0;
            },
            Key {
                key: KeyCode::Down,
                modifier: KeyModifiers::NONE,
            } => {
                self.index = self.text.text_len() - 1;
            },
            _ => {},
        }

        let mut callback = self.on_keypress.take();
        
        if let Some(ref mut callback) = callback {
            let call = callback.clone();
            call.borrow_mut()(self, key);
            self.on_keypress = Some(call);
        }

        
    }

}
