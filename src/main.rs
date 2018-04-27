#[macro_use]
extern crate lazy_static;
extern crate clap;

use clap::{App,Arg};

use std::collections::HashMap;
use std::io;
use std::io::prelude::*;
use std::fs::File;

mod grammar;
use grammar::*;

struct Compiler {
    cursor: u16,
    output: [u8; 0x10000],
    label_map: HashMap<Label, u16>,
    needs_label: Vec<(u16, Label)>,
}

impl Compiler {
    fn new() -> Self {
        Self {
            cursor: 0,
            output: [0; 0x10000],
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

    fn write_registers(&mut self, r0: Register, r1: Register) {
        self.write(&[ r0.0 << 4 | r1.0 ]);
    }

    fn process(&mut self, line: Line) {
        if let Some(label) = line.label {
            self.label_map.insert(label.clone(), self.cursor);
        }

        if let Some(instruction) = line.instruction {
            use grammar::Instruction::*;

            /* Write the binary output */
            match instruction {
                Db(vs) => {
                    for v in vs {
                        match v {
                            Serializable::Byte(b) => {
                                self.write(&[ b ]);
                            },
                            Serializable::String(s) => {
                                self.write(s.as_bytes());
                            },
                        }
                    }
                },
                Ds(len) => {
                    self.cursor += len;
                },
                Org(pos) => {
                    self.cursor = pos;
                },
                Nullary(opcode) => {
                    self.write(&[ opcode ]);
                },
                UnaryReg(opcode, register) => {
                    self.write(&[ opcode, register.0 ]);
                },
                UnaryAddr(opcode, address) => {
                    self.write(&[ opcode ]);
                    self.write_address(address);
                },
                BinaryRegIm(opcode, register, value) => {
                    self.write(&[ opcode, register.0, value ]);
                },
                BinaryRegReg(opcode, register0, register1) => {
                    self.write(&[ opcode ]);
                    self.write_registers(register0, register1);
                },
                BinaryRegAddr(opcode, register, address) => {
                    self.write(&[ opcode, register.0 ]);
                    self.write_address(address);
                },
                BinaryRegDeref(opcode, register, (high, low)) => {
                    self.write(&[ opcode, register.0 ]);
                    self.write_registers(high, low);
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

    fn compile(source: &str) -> Result<Vec<u8>, grammar::ParseError> {
        let mut compiler = Compiler::new();

        for line in program(&source)? {
            compiler.process(line);
        }

        compiler.resolve_labels();

        /* Strip trailing zeroes */
        let mut output = compiler.output.to_vec();
        while output.last() == Some(&0) {
            output.pop();
        }

        Ok(output)
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
            nop
            foo:
                nop
                jmp foo
        ");

        assert_eq!(binary, Ok(vec![ 0x00, 0x00, 0x00, 0x20, 0x00, 0x02 ]));
    }

    #[test]
    fn string_literals_are_zero_terminated() {
        let binary = Compiler::compile("
            .db 0xAA, \"a\", 0xBB
        ");

        assert_eq!(binary, Ok(vec![ 0xAA, 0x61, 0xBB ]));
    }
}

fn read_to_string<F: Read>(mut file: F) -> String {
    let mut buffer = String::new();
    file.read_to_string(&mut buffer).expect("Unable to read the file");
    buffer
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
        .arg(Arg::with_name("stdout")
            .long("stdout")
            .help("Output the resulting binary to stdout"))
        .get_matches();

    let filename = matches.value_of("file").expect("File name was not provided");

    let source = if filename == "-" {
        read_to_string(io::stdin())
    } else {
        read_to_string(File::open(filename).expect("Unable to open the file"))
    };

    match Compiler::compile(&source) {
        Ok(binary) => {
            let res = if matches.is_present("stdout") {
                io::stdout().write_all(&binary)
            } else {
                let outfile = matches.value_of("output").unwrap_or("out.bin");
                let mut file = File::create(outfile).expect("Failed to create output file");
                file.write_all(&binary)
            };

            res.expect("Failed to write file");
        },
        Err(err) => {
            println!("Error on {}:{}:{}, expected one of {:?}", filename, err.line, err.column, err.expected);
        }
    }
}
