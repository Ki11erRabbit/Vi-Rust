use std::io;


use crate::editor::Editor;

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


fn main() -> io::Result<()> {
    //let _cleanup = CleanUp;
    //terminal::enable_raw_mode()?;

    let mut editor = Editor::new();

    if let Some(filename) = std::env::args().nth(1) {
        editor.open_file(&filename)?;
    }
    eprintln!("Welcome to the editor!");

    while editor.run()? {}

    Ok(())
}
