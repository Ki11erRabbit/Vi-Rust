use std::{io, rc::Rc};


use crate::{editor::Editor, lsp::LspController};

pub mod window;
pub mod mode;
pub mod cursor;
pub mod settings;
pub mod pane;
pub mod buffer;
pub mod treesitter;
pub mod editor;
pub mod lsp;

//const EDITOR_NAME: &str = "vi";

#[tokio::main]
async fn main() -> io::Result<()> {
    //let _cleanup = CleanUp;
    //terminal::enable_raw_mode()?;

    eprintln!("Welcome to the editor!");


    let mut controller = LspController::new();

    let (lsp_sender, lsp_reciever) = std::sync::mpsc::channel();
    let (lsp_controller, lsp_controller_reciever) = std::sync::mpsc::channel();

    controller.set_listen(lsp_reciever);
    controller.set_response(lsp_controller);

    let lsp_listener = Rc::new(lsp_controller_reciever);


    let mut editor = Editor::new(lsp_sender, lsp_listener);

    if let Some(filename) = std::env::args().nth(1) {
        editor.open_file(&filename)?;
    }

    let tokio_handle = tokio::runtime::Handle::current();

    let handle = tokio_handle.spawn_blocking(move || {
        controller.run();
    });
    

    while editor.run()? {}

    handle.await.unwrap();

    Ok(())
}
