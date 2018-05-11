use std::io::prelude::*;
use std::fs::File;
use std::process;

pub fn read_to_string(filename: &str) -> String {
    let mut file = File::open(filename).unwrap_or_else(|err| {
        eprintln!("Failed to open file '{}': {}.", filename, err);
        process::exit(1);
    });

    let mut buffer = String::new();

    file.read_to_string(&mut buffer).unwrap_or_else(|err| {
        eprintln!("Failed to read file '{}': {}.", filename, err);
        process::exit(1);
    });

    buffer
}

pub fn write_to_file(filename: &str, contents: &[u8]) {
    let mut file = File::create(filename).unwrap_or_else(|err| {
        eprintln!("Failed to create file '{}': {}.", filename, err);
        process::exit(1);
    });

    file.write_all(contents).unwrap_or_else(|err| {
        eprintln!("Failed to write file '{}': {}.", filename, err);
    });
}
