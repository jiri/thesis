use std::collections::HashMap;
use std::fmt::Write;

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
                        self.write(&[0]);
                    }
                },
                Org(pos) => {
                    self.cursor = pos;
                },
                Add(r0, r1) => self.write(&vec![ 0x10, r0.0 << 4 | r1.0 ]),
                Addi(r0, i) => self.write(&vec![ 0x11, r0.0, ((i & 0xff00) >> 8) as u8, (i & 0x00ff >> 0) as u8 ]),
                Jmp(label) => {
                    self.needs_label.push((self.cursor + 2, label));
                    self.write(&vec![ 0x20, 0, 0, 0]);
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
}

fn main() {
    let source = r#"
        .org $10
            .db 01, 23, 45, 67, 89, AB, CD

        .org 0
            add R0, R1

        loop:
            addi R1, 12
            jmp loop
    "#;

    match program(source) {
        Ok(lines) => {
            let mut compiler = Compiler::new();

            for line in lines {
                compiler.process(line);
            }

            compiler.resolve_labels();

            let mut s = String::new();
            for &byte in compiler.output.iter() {
                write!(&mut s, "{:02x} ", byte).expect("Unable to write");
            }
            println!("{}", s);

            println!("{:#?}", compiler.label_map);

            // compiler.write_out("filename.bin");11
        },
        Err(e) => {
            println!("Parse error: {:#?}", e);
        },
    }
}
