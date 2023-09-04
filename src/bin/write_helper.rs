
use std::{env, fs::OpenOptions, io::{BufReader, BufRead, Write}};





fn main() {
    let args: Vec<String> = env::args().collect();

    let filename = &args[1];

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(filename)
        .expect("Could not open file");

    let mut reader = BufReader::new(std::io::stdin());

    let mut line = String::new();
    while let Ok(_) = reader.read_line(&mut line) {
        file.write_all(line.as_bytes()).expect("Could not write to file");
    }

    println!("Successful write to file");
}
