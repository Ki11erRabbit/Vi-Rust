use std::{io, rc::Rc};


use crate::{editor::Editor, lsp::{LspController, LspControllerMessage}};

pub mod window;
pub mod mode;
pub mod cursor;
pub mod settings;
pub mod pane;
pub mod buffer;
pub mod treesitter;
pub mod editor;
pub mod lsp;
pub mod registers;

//const EDITOR_NAME: &str = "vi";

pub trait Mailbox<M> {
    fn send(&self, message: M) -> Result<(), std::sync::mpsc::SendError<M>>;
    fn recv(&self) -> Result<M, std::sync::mpsc::RecvError>;
    fn try_recv(&self) -> Result<M, std::sync::mpsc::TryRecvError>;
}



fn main() -> io::Result<()> {
    //let _cleanup = CleanUp;
    //terminal::enable_raw_mode()?;

    eprintln!("Welcome to the editor!");


    let mut controller = LspController::new();

    let (lsp_sender, lsp_reciever) = std::sync::mpsc::channel();
    let (lsp_controller, lsp_controller_reciever) = std::sync::mpsc::channel();

    controller.set_listen(lsp_reciever);
    controller.set_response(lsp_controller);

    let lsp_listener = Rc::new(lsp_controller_reciever);


    let mut editor = Editor::new(lsp_sender.clone(), lsp_listener);


    let thread_handle = std::thread::spawn(move || {
        eprintln!("Starting LSP thread");
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        let tokio_handle = tokio_runtime.spawn_blocking(move || {
            eprintln!("Starting Tokio thread");
            let _ = controller.run();
            drop(controller);
        });
        eprintln!("Starting Tokio runtime");
        tokio_runtime.block_on(tokio_handle).unwrap();
    });


    
    if let Some(filename) = std::env::args().nth(1) {
        editor.open_file(&filename)?;
    }

    editor.draw()?;

    while editor.run()? {}

    lsp_sender.send(LspControllerMessage::Exit).unwrap();
    
    thread_handle.join().unwrap();

    Ok(())
}
