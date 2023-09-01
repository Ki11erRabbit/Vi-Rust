
fn main() {
    let language = "scheme";
    let package = format!("tree-sitter-{}", language);
    let source_directory = format!("{}/src", package);
    let source_file = format!("{}/parser.c", source_directory);

    println!("cargo:rerun-if-changed={}", source_file);

    cc::Build::new()
        .include(&source_directory)
        .file(source_file)
        .compile(&package);
}
