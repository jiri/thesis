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
    Dstr(String),
    Ds(u16),
    Org(u16),
    Nullary(Opcode),
    UnaryReg(Opcode, Register),
    UnaryAddr(Opcode, Address),
    BinaryRegIm(Opcode, Register, u8),
    BinaryRegReg(Opcode, Register, Register),
    BinaryRegAddr(Opcode, Register, Address),
    BinaryRegDeref(Opcode, Register, (Register, Register)),
}

#[derive(Debug)]
pub struct Line {
    pub label: Option<Label>,
    pub instruction: Option<Instruction>,
}

lazy_static! {
    static ref OPCODES: HashMap<&'static str, Opcode> = {
        let mut map = HashMap::new();

        /* Utility */
        map.insert("nop",   0x00);
        map.insert("stop",  0x01);
        map.insert("sleep", 0x02);
        map.insert("break", 0x03);
        map.insert("ei",    0x04);
        map.insert("di",    0x05);

        /* Arithmetic */
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

        /* Flow control */
        map.insert("jmp",   0x20);
        map.insert("call",  0x21);
        map.insert("ret",   0x22);
        map.insert("reti",  0x23);
        map.insert("brc",   0x24);
        map.insert("brnc",  0x25);
        map.insert("brz",   0x26);
        map.insert("brnz",  0x27);

        /* Load / store */
        map.insert("mov",   0x30);
        map.insert("ldi",   0x31);
        map.insert("ld",    0x32);
        map.insert("st",    0x33);
        map.insert("push",  0x34);
        map.insert("pop",   0x35);
        map.insert("lpm",   0x36);
        map.insert("ldd",   0x37);
        map.insert("std",   0x38);
        map.insert("lpmd",  0x39);
        map.insert("in",    0x3A);
        map.insert("out",   0x3B);

        map
    };
}

include!(concat!(env!("OUT_DIR"), "/gpr.rs"));
