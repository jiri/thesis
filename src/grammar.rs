pub type Label = String;

#[derive(Debug)]
pub struct Register(pub u8);

#[derive(Debug)]
pub enum Flag {
    Z,
    O,
}

#[derive(Debug)]
pub enum Address {
    Label(Label),
    Immediate(u16),
}

#[derive(Debug)]
pub enum Instruction {
    Db(Vec<u8>),
    Ds(u16),
    Org(u16),
    Nop,
    Mov(Register, Register),
    Movi(Register, u16),
    Add(Register, Register),
    Addi(Register, u16),
    Addc(Register, Register),
    Load(Register, Address),
    Store(Address, Register),
    Jmp(Address),
    Brif(Flag, Address),
    Brnif(Flag, Address),
}

impl Instruction {
    pub fn special(&self) -> bool {
        use Instruction::*;

        match self {
            Db(_) | Ds(_) | Org(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub struct Line {
    pub label: Option<Label>,
    pub instruction: Option<Instruction>,
}

impl Register {
    fn new(n: u8) -> Result<Register, &'static str> {
        if n <= 16 {
            Ok(Register(n))
        }
        else {
            Err("register index between 0 and 16")
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/grammar.rs"));
