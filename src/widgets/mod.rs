use std::{cell::RefCell, rc::Rc};

use crate::{settings::{Key, Settings, ColorScheme}, window::StyledChar};


pub mod text;
pub mod button;






pub trait Widget {
    /// This function is called to draw the widget.
    /// It takes in an index to draw a level and an output buffer.
    /// It returns true if the widget is done drawing.
    fn draw(&self, index: usize, output: &mut Vec<Option<StyledChar>>) -> bool;

    fn set_color_scheme(&mut self, scheme: ColorScheme);
}

pub trait InteractableWidget: Widget {
    /// This function is called when the widget is clicked on.
    fn set_on_click<F: FnMut(&mut Self) + 'static>(&mut self, callback: F);

    /// This function is called when the widget is focused on.
    fn set_on_focus<F: FnMut(&mut Self) + 'static>(&mut self, callback: F) where F: FnMut(&mut Self) + 'static;

    fn set_on_keypress<F: FnMut(&mut Self, Key) + 'static>(&mut self, callback: F);

    fn click(&mut self);

    fn focus(&mut self);

    fn keypress(&mut self, key: Key);
}

