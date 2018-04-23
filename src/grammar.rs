use std::collections::HashMap;

pub type Label = String;

#[derive(Debug)]
pub struct Register(pub u8);

impl Register {
    fn new(n: u8) -> Result<Register, &'static str> {
        if n <= 15 {
            Ok(Register(n))
        } else {
            Err("register index between 0 and 15")
        }
    }
}

#[derive(Debug)]
pub enum Address {
    Label(Label),
    Immediate(u16),
}

type Opcode = u8;

#[derive(Debug)]
pub enum Instruction {
    Db(Vec<u8>),
    Ds(u16),
    Org(u16),
    Nullary(Opcode),
    UnaryReg(Opcode, Register),
    UnaryAddr(Opcode, Address),
    BinaryRegIm(Opcode, Register, u8),
    BinaryRegReg(Opcode, Register, Register),
    BinaryRegAddr(Opcode, Register, Address),
}

#[derive(Debug)]
pub struct Line {
    pub label: Option<Label>,
    pub instruction: Option<Instruction>,
}

lazy_static! {
    static ref OPCODES: HashMap<&'static str, Opcode> = {
        let mut map = HashMap::new();

        map.insert("mov",   0x30);
        map.insert("ldi",   0x31);
        map.insert("ld",    0x32);
        map.insert("st",    0x33);
        map.insert("push",  0x34);
        map.insert("pop",   0x35);
        map.insert("lpm",   0x36);
        map.insert("add",   0x10);
        map.insert("addc",  0x11);
        map.insert("sub",   0x12);
        map.insert("subc",  0x13);
        map.insert("inc",   0x14);
        map.insert("dec",   0x15);
        map.insert("and",   0x16);
        map.insert("or",    0x17);
        map.insert("xor",   0x18);
        map.insert("cmp",   0x19);
        map.insert("jmp",   0x20);
        map.insert("call",  0x21);
        map.insert("ret",   0x22);
        map.insert("reti",  0x23);
        map.insert("brc",   0x24);
        map.insert("brnc",  0x25);
        map.insert("brz",   0x26);
        map.insert("brnz",  0x27);
        map.insert("nop",   0x00);
        map.insert("stop",  0x01);
        map.insert("sleep", 0x02);
        map.insert("break", 0x03);

        map
    };
}

include!(concat!(env!("OUT_DIR"), "/gpr.rs"));
