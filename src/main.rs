use std::io;
use std::env;

use crate::editor::Editor;

pub mod window;
pub mod mode;
pub mod cursor;
pub mod settings;
pub mod pane;
pub mod buffer;
pub mod treesitter;
pub mod editor;
pub mod utils;

fn update_path() {
    let path = env::var("PATH").unwrap();
    let mut paths = env::split_paths(&path).collect::<Vec<_>>();
    paths.push("/home/ki11errabbit/Documents/Programing-Projects/Rust/Vi-Rust/target/debug".into());
    paths.push("/home/ki11errabbit/Documents/Programing-Projects/Rust/Vi-Rust/target/release".into());
    let new_path = env::join_paths(paths).unwrap();
    env::set_var("PATH", &new_path);

}


fn main() -> io::Result<()> {
    update_path();
    //let _cleanup = CleanUp;
    //terminal::enable_raw_mode()?;

    let mut editor = Editor::new();

    if let Some(filename) = std::env::args().nth(1) {
        editor.open_file(&filename)?;
    }

    while editor.run()? {}

    Ok(())
}
