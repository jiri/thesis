use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use serde_json;

use grammar::*;
use util::read_to_string;

pub struct Compiler {
    cursor: u16,
    output: [u8; 0x10000],
    label_map: HashMap<Label, u16>,
    needs_label: Vec<(u16, Label, Nibble)>,
    last_major_label: Label,
    enabled_instructions: Option<HashMap<Opcode, String>>,
    file_stack: FileStack,
}

struct FileStack {
    filenames: Vec<String>,
    lines: Vec<Vec<(usize, String)>>,
}

impl FileStack {
    fn new() -> Self {
        Self {
            filenames: Vec::new(),
            lines: Vec::new(),
        }
    }

    fn init(&mut self, file: &str, lines: Vec<(usize, String)>) {
        self.filenames.push(file.to_owned());
        self.lines.push(lines);
    }

    fn push(&mut self, file: &str) -> Result<(), String> {
        assert!(!self.filenames.is_empty());
        assert_eq!(self.filenames.len(), self.lines.len());

        let filepath: String = {
            let mut path = PathBuf::from(file);

            if !path.is_file() {
                return Err(format!("Path '{}' doesn't point to a file.", file));
            }

            if !path.is_absolute() {
                let current_dir = Path::new(self.filenames.last().unwrap()).parent().unwrap();
                path = current_dir.join(path);
            }

            path.to_str().unwrap().to_owned()
        };

        if self.filenames.contains(&filepath) {
            return Err(format!("Recursive inclusion detected in file '{}'.", file));
        }

        let lines = read_to_string(file).split('\n')
            .enumerate()
            .map(|(i, x)| (i + 1, x.to_owned()))
            .collect::<Vec<(usize, String)>>().into_iter()
            .rev()
            .collect();

        self.filenames.push(filepath);
        self.lines.push(lines);

        Ok(())
    }

    fn pop(&mut self) -> Option<(String, (usize, String))> {
        if self.filenames.is_empty() {
            None
        }
        else if let Some(line) = self.lines.last_mut().and_then(|x| x.pop()) {
            let filename = self.filenames.last_mut().expect("Inconsistent state in FileStack");
            Some((filename.clone(), line))
        }
        else {
            self.filenames.pop();
            self.lines.pop();
            self.pop()
        }
    }
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
            file_stack: FileStack::new(),
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

    pub fn compile_file(filename: &str, whitelist: Option<Vec<String>>) -> Result<(Vec<u8>, String), String> {
        let source = read_to_string(filename);
        Self::compile(filename, &source, whitelist)
    }

    #[allow(dead_code)]
    pub fn compile_source(source: &str, whitelist: Option<Vec<String>>) -> Result<(Vec<u8>, String), String> {
        Self::compile("-", source, whitelist)
    }

    fn compile(filename: &str, source: &str, whitelist: Option<Vec<String>>) -> Result<(Vec<u8>, String), String> {
        let mut compiler = Compiler::new();

        if let Some(mnemonics) = whitelist {
            let mut map = HashMap::new();

            for mnemonic in mnemonics {
                if let Some(opcode) = OPCODES.get(mnemonic.as_str()) {
                    map.insert(*opcode, mnemonic);
                }
                else {
                    return Err(format!("Unknown whitelist instruction '{}' in file '{}'", mnemonic, filename));
                }
            }

            compiler.enabled_instructions = Some(map);
        }

        let init_lines = source.split('\n')
            .enumerate()
            .map(|(i, x)| (i + 1, x.to_owned()))
            .collect::<Vec<(usize, String)>>().into_iter()
            .rev()
            .collect();

        compiler.file_stack.init(filename, init_lines);

        while let Some((file, (ln, line))) = compiler.file_stack.pop() {
            match parse_line(&line) {
                Ok(l) => {
                    if let Some(Instruction::Include(path)) = l.instruction {
                        compiler.file_stack.push(&path)?;
                    }
                    else {
                        compiler.process(l)?
                    }
                },
                Err(mut e) => {
                    let first = e.expected.iter().nth(0).unwrap().clone();
                    if e.expected.len() == 1 {
                        return Err(format!("In {}:{}:{}, expected {}", file, ln, e.column, first));
                    }
                        else {
                            let rest: Vec<&str> = e.expected.iter().skip(1).cloned().collect();
                            return Err(format!("In {}:{}:{}, expected {} or {}", file, ln, e.column, rest.join(", "), first));
                        }
                },
            }
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
        let binary = Compiler::compile_source("add R0, R1", None).expect("Failed to compile code");

        assert_eq!(binary.0, vec![ 0x10, 0x01 ]);
    }

    #[test]
    fn it_resolves_labels() {
        let binary = Compiler::compile_source("
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
        let binary = Compiler::compile_source("
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
        let binary = Compiler::compile_source("
            db 0xAA, \"a\", 0xBB
        ", None).expect("Failed to compile code");

        assert_eq!(binary.0, vec![ 0xAA, 0x61, 0xBB ]);
    }

    #[test]
    fn it_produces_a_symfile() {
        let binary = Compiler::compile_source("
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
        let binary = Compiler::compile_source("
            add R0, R1
            sub R0, R1
        ", Some(vec![ "add".to_owned() ]));

        assert!(binary.is_err());
    }

    #[test]
    fn it_resolves_high_low_addr() {
        let binary = Compiler::compile_source("
            org 0x0
            ldi R0, hi(addr)
            ldi R1, lo(addr)
            org 0xABBA
            addr:
        ", None).expect("Failed to compile code");

        assert_eq!(binary.0, vec![ 0x31, 0x00, 0xAB, 0x31, 0x01, 0xBA ]);
    }
}
