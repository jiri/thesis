extern crate clap;

use clap::{App,Arg};

use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::File;

mod grammar;
use grammar::*;

struct Compiler {
    cursor: u16,
    output: [u8; 32],
    label_map: HashMap<Label, u16>,
    needs_label: Vec<(u16, Label)>,
}

impl Compiler {
    fn new() -> Self {
        Self {
            cursor: 0,
            output: [0; 32],
            label_map: HashMap::new(),
            needs_label: Vec::new(),
        }
    }

    fn write(&mut self, bs: &[u8]) {
        self.output[self.cursor as usize .. self.cursor as usize + bs.len()].clone_from_slice(bs);
        self.cursor += bs.len() as u16;
    }

    fn write_word(&mut self, word: u16) {
        let hi_byte = ((word & 0xFF00) >> 8) as u8;
        let lo_byte = ((word & 0x00FF) >> 0) as u8;
        self.write(&[ hi_byte, lo_byte ]);
    }

    fn write_address(&mut self, addr: Address) {
        match addr {
            Address::Label(label) => {
                self.needs_label.push((self.cursor, label));
                self.write_word(0x0000);
            },
            Address::Immediate(i) => {
                self.write_word(i);
            },
        }
    }

    fn process(&mut self, line: Line) {
        if let Some(label) = line.label {
            self.label_map.insert(label, self.cursor);
        }

        if let Some(instruction) = line.instruction {
            use grammar::Instruction::*;

            match instruction {
                Db(bs) => {
                    self.write(&bs);
                    // Align to word
                    if bs.len() % 2 == 1 {
                        self.write(&[ 0 ]);
                    }
                },
                Ds(len) => {
                    self.cursor += len;
                },
                Org(pos) => {
                    self.cursor = pos;
                },
                Nop => {
                    self.write(&[ 0x00, 0x00 ])
                },
                Mov(r0, r1) => {
                    self.write(&[ 0x01, r0.0 << 4 | r1.0 ])
                },
                Movi(r0, i) => {
                    self.write(&[ 0x02, r0.0 ]);
                    self.write_word(i);
                },
                Add(r0, r1) => {
                    self.write(&[ 0x10, r0.0 << 4 | r1.0 ])
                },
                Addi(r0, i) => {
                    self.write(&[ 0x11, r0.0 ]);
                    self.write_word(i);
                },
                Addc(r0, r1) => {
                    self.write(&[ 0x12, r0.0 << 4 | r1.0 ])
                },
                Load(r0, addr) => {
                    self.write(&[ 0x30, r0.0 ]);
                    self.write_address(addr);
                },
                Store(addr, r0) => {
                    self.write(&[ 0x31, r0.0 ]);
                    self.write_address(addr);
                },
                Jmp(addr) => {
                    self.write(&[ 0x20, 0x00 ]);
                    self.write_address(addr);
                },
            }
        }
    }

    fn resolve_labels(&mut self) {
        for (position, label) in self.needs_label.iter() {
            if let Some(addr) = self.label_map.get(label) {
                self.output[*position as usize .. *position as usize + 2].clone_from_slice(&vec![ ((addr & 0xff00) >> 8) as u8, (addr & 0x00ff >> 0) as u8 ]);
            }
            else {
                println!("Warning: Undefined label '{}'!", label);
            }
        }
    }

    fn compile(source: &str) -> Result<Vec<u8>, String> {
        match program(&source) {
            Ok(lines) => {
                let mut compiler = Compiler::new();

                for line in lines {
                    compiler.process(line);
                }

                compiler.resolve_labels();

                /* Strip trailing zeroes */
                let mut output = compiler.output.to_vec();
                while output.last() == Some(&0) {
                    output.pop();
                }

                Ok(output)
            },
            Err(e) => {
                Err(format!("{:#?}", e))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_produces_output() {
        let binary = Compiler::compile("add R0, R1");

        assert_eq!(binary, Ok(vec![ 0x10, 0x01 ]));
    }

    #[test]
    fn it_resolves_labels() {
        let binary = Compiler::compile("
            nop
            foo:
                nop
                jmp foo
        ");

        assert_eq!(binary, Ok(vec![ 0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x02 ]));
    }
}

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
        .get_matches();

    let filename = matches.value_of("file").expect("File name was not provided");
    let mut file = File::open(filename).expect("Unable to open the file");
    let mut source = String::new();
    file.read_to_string(&mut source).expect("Unable to read the file");

    match Compiler::compile(&source) {
        Ok(binary) => {
            let outfile = matches.value_of("output").unwrap_or("out.bin");
            let mut file = File::create(outfile).expect("Failed to create output file");
            file.write_all(&binary).expect("Failed to write file");
        },
        Err(e) => {
            println!("{}", e);
        }
    }
}
