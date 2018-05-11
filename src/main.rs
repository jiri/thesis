#[macro_use]
extern crate lazy_static;
extern crate clap;

extern crate serde;
extern crate serde_json;

mod grammar;
mod compiler;
mod util;

use clap::{App,Arg};

use compiler::*;
use util::{read_to_string,write_to_file};

fn main() {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("file")
            .value_name("FILE")
            .help("Path to the source file")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("output")
            .value_name("OUTPUT")
            .short("o")
            .long("output")
            .help("Path to the output file")
            .required(false)
            .takes_value(true))
        .arg(Arg::with_name("symfile")
            .value_name("FILE")
            .short("s")
            .long("symfile")
            .help("If set, path where the symfile will be outputted")
            .required(false)
            .takes_value(true))
        .arg(Arg::with_name("whitelist")
            .value_name("FILE")
            .short("w")
            .long("whitelist")
            .help("If set, path to a file containing instruction whitelist")
            .required(false)
            .takes_value(true))
        .get_matches();

    let source = read_to_string(matches.value_of("file").expect("File name was not provided"));

    let whitelist: Option<Vec<String>> =
        matches.value_of("whitelist")
            .map(read_to_string)
            .map(|ref s| {
                serde_json::from_str(s)
                    .unwrap_or_else(|err| {
                        eprintln!("Failed to parse whitelist: {}.", err);
                        std::process::exit(1);
                    })
            });

    match Compiler::compile(&source, whitelist) {
        Ok((binary, symbols)) => {
            write_to_file(matches.value_of("output").unwrap_or("out.bin"), &binary);

            if let Some(symfilepath) = matches.value_of("symfile") {
                write_to_file(symfilepath, symbols.as_bytes());
            }
        },
        Err(err) => {
            println!("Error: {}", err);
        }
    }
}
