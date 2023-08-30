use std::{rc::Rc, cell::RefCell, sync::mpsc::Sender};

use super::PaneMessage;








pub struct PopUpPane {
    mode : Rc<RefCell<dyn PopUp>>,
    sender: Sender<PaneMessage>,
    prompt: String,
    input: String,
}


