use fmt::Formatter;
use std::{fmt, intrinsics::transmute, slice::Iter};

pub type Number = f64;
pub type ConstantIndex = u8;

#[derive(Debug, Clone)]
#[repr(u8)]
pub enum ByteCode {
    Return,
    Constant,
    Negate,
}

#[derive(Debug)]
pub enum Instruction {
    Return,
    Constant(ConstantIndex),
    Negate,
}

pub struct Chunk {
    code: Vec<u8>,
    lines: Vec<usize>,
    values: Vec<Value>,
}

#[derive(Clone, Debug, Copy)]
pub enum Value {
    Number(Number),
}

pub struct ChunkIterator<'a>(Iter<'a, u8>);

impl Iterator for ChunkIterator<'_> {
    type Item = Instruction;
    fn next(&mut self) -> Option<Self::Item> {
        let byte_code: ByteCode = (*self.0.next()?).into();
        match byte_code {
            ByteCode::Return => Some(Instruction::Return),
            ByteCode::Constant => Some(Instruction::Constant(*self.0.next()?)),
            ByteCode::Negate => Some(Instruction::Negate),
        }
    }
}

// impl From<&Instruction> for ByteCode {
//     fn from(instr: &Instruction) -> Self {
//         match instr {
//             Instruction::Return => ByteCode::Return,
//             Instruction::Constant(_) => ByteCode::Constant
//         }
//     }
// }

impl From<u8> for ByteCode {
    fn from(byte: u8) -> Self {
        unsafe { transmute::<_, Self>(byte) }
    }
}

impl From<ByteCode> for u8 {
    fn from(byte_code: ByteCode) -> Self {
        byte_code as u8
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(num) => write!(f, "{}", num),
        }
    }
}

impl fmt::Display for ByteCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for Instruction {
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

    pub fn add_instruction(&mut self, instr: Instruction, line: usize) {
        self.lines.push(line);

        match instr {
            Instruction::Return => self.code.push(ByteCode::Return.into()),
            Instruction::Constant(const_index) => {
                self.code.push(ByteCode::Constant.into());
                self.code.push(const_index);
            }
            Instruction::Negate => self.code.push(ByteCode::Negate.into()),
        }
    }

    pub fn add_value(&mut self, value: Value) -> u8 {
        self.values.push(value);
        (self.values.len() - 1) as u8
    }

    pub fn get_value(&self, index: u8) -> Value {
        self.values[index as usize]
    }

    pub fn instr_iter(&self) -> ChunkIterator {
        ChunkIterator(self.code.iter())
    }

    pub fn disassemble_instruction(&self, index: usize, instr: &Instruction) -> String {
        let line_str = if index == 0 || self.lines[index] != self.lines[index - 1] {
            self.lines[index].to_string()
        } else {
            "|".to_owned()
        };

        let mut extension = "".to_owned();

        match instr {
            Instruction::Constant(const_index) => {
                extension = self.values[*const_index as usize].to_string();
            }
            _ => {}
        };

        return format!("{:0>4} {: >4} {} {}", index, line_str, instr, extension);
    }
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut instrs = "".to_owned();
        let mut chunk_iter = self.instr_iter().enumerate();
        while let Some((index, instruction)) = chunk_iter.next() {
            let opcode_view = self.disassemble_instruction(index, &instruction);
            instrs.push_str(&opcode_view);
            instrs.push('\n');
        }

        write!(f, "{}", instrs)
    }
}
