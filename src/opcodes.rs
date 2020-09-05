use fmt::Formatter;
use std::{fmt, intrinsics::transmute};

pub type Number = f64;

#[derive(Debug)]
#[repr(u8)]
pub enum OpCode {
    Return,
    Constant,
}

pub struct Chunk {
    code: Vec<u8>,
    lines: Vec<usize>,
    values: Vec<Value>,
}

pub enum Value {
    Number(Number),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(num) => write!(f, "{}", num),
        }
    }
}

impl fmt::Display for OpCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: Vec::new(),
            lines: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn disassemble(&self, chunk_name: &str) -> String {
        format!(
            "== {} ==
{}",
            chunk_name, self
        )
    }

    // This assumes that the u8 opcode is valid according to the OpCode to discriminator mapping
    pub fn disassemble_instruction(&self, offset: usize) -> (String, usize) {
        let opcode = unsafe { transmute::<u8, OpCode>(self.code[offset]) };
        let mut new_offset = offset + 1;
        let mut extension = "".to_owned();
        match opcode {
            OpCode::Constant => {
                let const_index = self.code[offset + 1];
                let const_val = &self.values[const_index as usize];
                extension = format!(" {} {}", const_index, const_val);
                new_offset += 1;
            }
            _ => {}
        };
        let line_num = if offset == 0 || self.lines[offset] != self.lines[offset - 1] {
            self.lines[offset].to_string()
        } else {
            "|".to_string()
        };
        (
            format!("{:0>4} {: >4} {}{}", offset, line_num, opcode, extension),
            new_offset,
        )
    }

    pub fn add_instruction(&mut self, opcode: OpCode, line: usize) {
        self.add_byte(opcode as u8, line)
    }

    pub fn add_byte(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn add_value(&mut self, value: Value) -> u8 {
        self.values.push(value);
        (self.values.len() - 1) as u8
    }
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut offset = 0;
        let mut instrs = "".to_owned();
        while offset < self.code.len() {
            let (instruction, new_offset) = self.disassemble_instruction(offset);
            instrs.push_str(&instruction);
            instrs.push('\n');
            offset = new_offset;
        }

        write!(f, "{}\n", instrs)
    }
}
