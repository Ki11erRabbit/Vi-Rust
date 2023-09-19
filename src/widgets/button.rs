use std::{cell::RefCell, rc::Rc};

use crossterm::event::KeyCode;

use crate::settings::ColorScheme;

use super::{text::Label, Widget, InteractableWidget};








pub struct Button {
    widget: Box<dyn InteractableWidget>,
    enabled: bool,
    focused: bool,
    on_click: Option<Rc<RefCell<dyn FnMut(&mut Self)>>>,
    on_focus: Option<Rc<RefCell<dyn FnMut(&mut Self)>>>,
    on_keypress: Option<Rc<RefCell<dyn FnMut(&mut Self, crate::settings::Key)>>>,
}


impl Button {
    pub fn new(mut widget: Box<dyn InteractableWidget>, enabled: bool) -> Self {

        widget.set_color_scheme(ColorScheme::ui());
        
        Self {
            widget,
            enabled,
            focused: false,
            on_click: None,
            on_focus: None,
            on_keypress: None,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }
}


impl Widget for Button {
    fn draw(&self, index: usize, output: &mut Vec<Option<crate::window::StyledChar>>) -> bool {
        self.widget.draw(index, output)
    }

    fn set_color_scheme(&mut self, scheme: ColorScheme) {
        self.widget.set_color_scheme(scheme);
    }
}


impl InteractableWidget for Button {
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
        if !self.enabled {
            return;
        }
        let callback = self.on_click.take();
        if let Some(callback) = callback {
            let call = callback.clone();
            callback.borrow_mut()(self);
            self.on_click = Some(call);
        }
    }

    fn focus(&mut self) {
        let callback = self.on_focus.take();
        if let Some(callback) = callback {
            let call = callback.clone();
            callback.borrow_mut()(self);
            self.on_focus = Some(call);
        }
    }

    fn keypress(&mut self, key: crate::settings::Key) {
        match &key {
            crate::settings::Key {
                key: KeyCode::Char(' '),
                ..
            } => {
                self.click();
            }
            crate::settings::Key {
                key: KeyCode::Enter,
                ..
            } => {
                self.click();
            }
            _ => {}
        }

        
        let callback = self.on_keypress.take();
        if let Some(callback) = callback {
            let call = callback.clone();
            callback.borrow_mut()(self, key);
            self.on_keypress = Some(call);
        }
    }
}

