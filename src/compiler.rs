use std::collections::HashMap;

use serde_json;

use grammar::*;
use std::io::Read;
use std::fs::File;

pub struct Compiler {
    cursor: u16,
    output: [u8; 0x10000],
    label_map: HashMap<Label, u16>,
    needs_label: Vec<(u16, Label, Nibble)>,
    last_major_label: Label,
    enabled_instructions: Option<HashMap<Opcode, String>>,
    file_stack: Vec<Vec<String>>,
}

impl Compiler {
    fn new() -> Self {
        Self {
            cursor: 0,
            output: [0; 0x10000],
            label_map: HashMap::new(),
            needs_label: Vec::new(),
            last_major_label: String::new(),
            enabled_instructions: None,
            file_stack: Vec::new(),
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
                if label.starts_with(".") {
                    self.needs_label.push((self.cursor, self.last_major_label.clone() + &label, Nibble::Both));
                } else {
                    self.needs_label.push((self.cursor, label, Nibble::Both));
                }
                self.write_word(0x0000);
            },
            Address::Immediate(i) => {
                self.write_word(i);
            },
        }
    }

    fn write_value(&mut self, value: Value) {
        match value {
            Value::Immediate(v) => {
                self.write(&[ v ]);
            },
            Value::Addr(addr, nib) => {
                match addr {
                    Address::Label(label) => {
                        if label.starts_with(".") {
                            self.needs_label.push((self.cursor, self.last_major_label.clone() + &label, nib));
                        } else {
                            self.needs_label.push((self.cursor, label, nib));
                        }
                        self.write(&[ 0x00 ]);
                    },
                    Address::Immediate(i) => {
                        match nib {
                            Nibble::Both => unreachable!(),
                            Nibble::High => {
                                let hi_byte = ((i & 0xFF00) >> 8) as u8;
                                self.write(&[ hi_byte ]);
                            },
                            Nibble::Low => {
                                let lo_byte = ((i & 0x00FF) >> 0) as u8;
                                self.write(&[ lo_byte ]);
                            },
                        }
                    },
                }
            },
        }
    }

    fn write_registers(&mut self, r0: Register, r1: Register) {
        self.write(&[ r0.0 << 4 | r1.0 ]);
    }

    fn write_serializable(&mut self, value: Serializable) {
        match value {
            Serializable::Byte(b)   => self.write(&[ b ]),
            Serializable::String(s) => self.write(s.as_bytes()),
        }
    }

    fn process(&mut self, line: Line) -> Result<(), String> {
        if let Some(label) = line.label {
            if label.chars().next().unwrap().is_uppercase() {
                self.last_major_label = label.clone();
            }

            if label.starts_with(".") {
                self.label_map.insert(self.last_major_label.clone() + &label, self.cursor);
            } else {
                self.label_map.insert(label.clone(), self.cursor);
            }
        }

        if let Some(instruction) = line.instruction {
            use grammar::Instruction::*;

            if let Some(opcode) = instruction.opcode() {
                if let Some(ref whitelist) = self.enabled_instructions {
                    if !whitelist.contains_key(&opcode) {
                        for (mnem, op) in OPCODES.iter() {
                            if *op == opcode {
                                return Err(format!("Use of instruction '{}' not allowed with current whitelist", mnem));
                            }
                        }
                        panic!("Opcode set changed between parsing and processing.");
                    }
                }
            }

            /* Write the binary output */
            match instruction {
                Db(vs) => {
                    for v in vs {
                        self.write_serializable(v);
                    }
                },
                Ds(len) => {
                    self.cursor += len;
                },
                Org(pos) => {
                    self.cursor = pos;
                },
                Include(_) => {
                    panic!("Processing include in Compiler::process!");
                }
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
                    self.write(&[ opcode, register.0 ]);
                    self.write_value(value);
                },
                BinaryRegReg(opcode, register0, register1) => {
                    self.write(&[ opcode ]);
                    self.write_registers(register0, register1);
                },
            }
        }

        Ok(())
    }

    fn read_to_string<F: Read>(mut file: F) -> String {
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).expect("Unable to read the file");
        buffer
    }

    pub fn compile(source: &str, whitelist: Option<Vec<String>>) -> Result<(Vec<u8>, String), String> {
        let mut compiler = Compiler::new();

        if let Some(mnemonics) = whitelist {
            let mut map = HashMap::new();

            for mnemonic in mnemonics {
                if let Some(opcode) = OPCODES.get(mnemonic.as_str()) {
                    map.insert(*opcode, mnemonic);
                }
                else {
                    return Err(format!("Unknown whitelist instruction '{}'", mnemonic));
                }
            }

            compiler.enabled_instructions = Some(map);
        }

        compiler.file_stack = vec![ source.split('\n').rev().map(|x| x.to_owned()).collect() ];

        while !compiler.file_stack.is_empty() {
            while let Some(line) = compiler.file_stack.last_mut().and_then(|x| x.pop()) {
                match parse_line(&line) {
                    Ok(l) => {
                        if let Some(Instruction::Include(path)) = l.instruction {
                            let lines = Self::read_to_string(File::open(path).unwrap()).split('\n').rev().map(|x| x.to_owned()).collect();
                            compiler.file_stack.push(lines);
                        }
                        else {
                            compiler.process(l)?
                        }
                    },
                    Err(e) => {
                        return Err(format!("On {}:{}, expected one of {:?}", e.line, e.column, e.expected));
                    },
                }
            }
            compiler.file_stack.pop();
        }

        compiler.resolve_labels()?;

        /* Strip trailing zeroes */
        let mut output = compiler.output.to_vec();
        while output.last() == Some(&0) {
            output.pop();
        }

        Ok((output, serde_json::to_string(&compiler.label_map).unwrap()))
    }

    fn resolve_labels(&mut self) -> Result<(), String> {
        for (position, label, nib) in self.needs_label.iter() {
            let addr = self.label_map.get(label).ok_or(format!("Undefined label '{}'!", label))?;

            match nib {
                Nibble::Both => {
                    self.output[*position as usize + 0] = ((addr & 0xff00) >> 8) as u8;
                    self.output[*position as usize + 1] = ((addr & 0x00ff) >> 0) as u8;
                },
                Nibble::High => {
                    self.output[*position as usize] = ((addr & 0xff00) >> 8) as u8;
                },
                Nibble::Low => {
                    self.output[*position as usize] = ((addr & 0x00ff) >> 0) as u8;
                },
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_produces_output() {
        let binary = Compiler::compile("add R0, R1", None).expect("Failed to compile code");

        assert_eq!(binary.0, vec![ 0x10, 0x01 ]);
    }

    #[test]
    fn it_resolves_labels() {
        let binary = Compiler::compile("
            nop
            nop
            foo:
                nop
                jmp foo
        ", None).expect("Failed to compile code");

        assert_eq!(binary.0, vec![ 0x00, 0x00, 0x00, 0x20, 0x00, 0x02 ]);
    }

    #[test]
    fn it_resolves_local_labels() {
        let binary = Compiler::compile("
            First:
            .loop:
                jmp .loop

            Second:
            .loop:
                jmp .loop
        ", None).expect("Failed to compile code");

        assert_eq!(binary.0, vec![ 0x20, 0x00, 0x00, 0x20, 0x00, 0x03 ]);
    }

    #[test]
    fn string_literals_are_not_zero_terminated() {
        let binary = Compiler::compile("
            db 0xAA, \"a\", 0xBB
        ", None).expect("Failed to compile code");

        assert_eq!(binary.0, vec![ 0xAA, 0x61, 0xBB ]);
    }

    #[test]
    fn it_produces_a_symfile() {
        let binary = Compiler::compile("
            org 0x0
            A:
            org 0x100
            B:
            org 0x40
            C:
        ", None).expect("Failed to compile code");

        let syms: HashMap<String, u16> = serde_json::from_str(&binary.1).expect("Failed to read symfile as json");

        assert_eq!(syms["A"], 0x0);
        assert_eq!(syms["B"], 0x100);
        assert_eq!(syms["C"], 0x40);
    }

    #[test]
    fn it_respects_whitelist() {
        let binary = Compiler::compile("
            add R0, R1
            sub R0, R1
        ", Some(vec![ "add".to_owned() ]));

        assert!(binary.is_err());
    }

    #[test]
    fn it_resolves_high_low_addr() {
        let binary = Compiler::compile("
            org 0x0
            ldi R0, hi(addr)
            ldi R1, lo(addr)
            org 0xABBA
            addr:
        ", None).expect("Failed to compile code");

        assert_eq!(binary.0, vec![ 0x31, 0x00, 0xAB, 0x31, 0x01, 0xBA ]);
    }
}
