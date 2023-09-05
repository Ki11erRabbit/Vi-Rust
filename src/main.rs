use std::{io, process::{Command, Stdio}, time::Duration, thread};


use crate::editor::Editor;

pub mod window;
pub mod mode;
pub mod cursor;
pub mod settings;
pub mod pane;
pub mod buffer;
pub mod treesitter;
pub mod editor;
pub mod lsp_client;




fn main() -> io::Result<()> {
    //let _cleanup = CleanUp;
    //terminal::enable_raw_mode()?;

    let clangd = Command::new("clangd")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn clangd");

    let mut lsp_client = lsp_client::LspClient::new(clangd.stdin.unwrap(), clangd.stdout.unwrap());

    lsp_client.initialize()?;

    thread::sleep(Duration::from_millis(100));
    
    lsp_client.process_messages()?;

    let mut editor = Editor::new();

    if let Some(filename) = std::env::args().nth(1) {
        editor.open_file(&filename)?;
    }

    while editor.run()? {}

    Ok(())
}
