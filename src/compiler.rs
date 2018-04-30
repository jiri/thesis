use std::collections::HashMap;

use serde_json;

use grammar;
use grammar::*;

pub struct Compiler {
    cursor: u16,
    output: [u8; 0x10000],
    label_map: HashMap<Label, u16>,
    needs_label: Vec<(u16, Label)>,
    last_major_label: Label,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            output: [0; 0x10000],
            label_map: HashMap::new(),
            needs_label: Vec::new(),
            last_major_label: String::new(),
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
                    self.needs_label.push((self.cursor, self.last_major_label.clone() + &label));
                } else {
                    self.needs_label.push((self.cursor, label));
                }
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

    pub fn compile(source: &str) -> grammar::ParseResult<(Vec<u8>, String)> {
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

        Ok((output, serde_json::to_string(&compiler.label_map).unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_produces_output() {
        let binary = Compiler::compile("add R0, R1").expect("Failed to compile code");

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
        ").expect("Failed to compile code");

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
        ").expect("Failed to compile code");

        assert_eq!(binary.0, vec![ 0x20, 0x00, 0x00, 0x20, 0x00, 0x03 ]);
    }

    #[test]
    fn string_literals_are_not_zero_terminated() {
        let binary = Compiler::compile("
            .db 0xAA, \"a\", 0xBB
        ").expect("Failed to compile code");

        assert_eq!(binary.0, vec![ 0xAA, 0x61, 0xBB ]);
    }

    #[test]
    fn it_produces_a_symfile() {
        let binary = Compiler::compile("
            .org 0x0
            A:
            .org 0x100
            B:
            .org 0x40
            C:
        ").expect("Failed to compile code");

        let syms: HashMap<String, u16> = serde_json::from_str(&binary.1).expect("Failed to read symfile as json");

        assert_eq!(syms["A"], 0x0);
        assert_eq!(syms["B"], 0x100);
        assert_eq!(syms["C"], 0x40);
    }
}
