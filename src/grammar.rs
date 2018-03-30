pub type Label = String;

#[derive(Debug)]
pub struct Register(pub u8);

#[derive(Debug)]
pub enum Instruction {
    Db(Vec<u8>),
    Org(u16),
    Add(Register, Register),
    Addi(Register, u16),
    Jmp(Label),
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
