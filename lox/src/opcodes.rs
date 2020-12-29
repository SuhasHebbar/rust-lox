use fmt::{Display, Formatter};
use std::{convert::{TryFrom, TryInto}, error::Error, fmt, rc::Rc};

pub type Number = f64;
pub type ConstantIndex = u8;

trait ByteCodeEncodeDecode: Sized {
    fn encode(&self, dest: &mut Vec<u8>);
    fn decode(src: &[u8]) -> (Self, &[u8]);
}
use lox_macros::ByteCodeEncodeDecode;

use crate::gc::Gc;

#[derive(Debug, ByteCodeEncodeDecode)]
pub enum Instruction {
    Return,
    Constant(ConstantIndex),

    Negate,
    Not,
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    Greater,
    Less,

    // Dedicated literal loads
    Nil,
    True,
    False,
}

pub struct Chunk {
    code: Vec<u8>,
    lines: Vec<usize>,
    values: Vec<Value>,
}

#[derive(Clone, Debug)]
pub enum Value {
    Nil,
    Number(Number),
    Boolean(bool),
    String(Gc<String>)
}

impl From<Number> for Value {
    fn from(val: Number) -> Self {
        Value::Number(val)
    }
}

impl From<bool> for Value {
    fn from(val: bool) -> Self {
        Value::Boolean(val)
    }
}

impl From<Gc<String>> for Value {
    fn from(val: Gc<String>) -> Self {
        Value::String(val)
    }
}

impl From<String> for Value {
    fn from(val: String) -> Self {
        Value::String(val.into())
    }
}


pub struct ChunkIterator<'a>(&'a [u8]);

impl Iterator for ChunkIterator<'_> {
    type Item = Instruction;
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            None
        } else {
            let (instr, tmp) = Instruction::decode(self.0);
            self.0 = tmp;

            Some(instr)
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Number(num) => write!(f, "{}", num),
            Value::Boolean(val) => write!(f, "{}", val),
            Value::String(string) => write!(f, "'{}'", string),
        }
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

    // TODO: Add method to add multiple instructions. Maybe reserve space in vector in advance.

    pub fn add_instruction(&mut self, instr: Instruction, line: usize) {
        self.lines.push(line);
        instr.encode(&mut self.code);
    }

    pub fn add_value(&mut self, value: Value) -> u8 {
        self.values.push(value);
        (self.values.len() - 1) as u8
    }

    pub fn get_value(&self, index: u8) -> &Value {
        &self.values[index as usize]
    }

    pub fn instr_iter(&self) -> ChunkIterator {
        ChunkIterator(&self.code[..])
    }

    pub fn disassemble_instruction(&self, index: usize, instr: &Instruction) -> String {
        let line_str = if index == 0 || self.lines[index] != self.lines[index - 1] {
            self.lines[index].to_string()
        } else {
            "|".to_owned()
        };

        let extension = match instr {
            Instruction::Constant(const_index) => {
                self.values[*const_index as usize].to_string()
            }
            _ => {"".to_owned()}
        };

        return format!("{:0>4} {: >4} {} {}", index, line_str, instr, extension);
    }

    pub fn get_line(&self, instr_index: usize) -> usize {
        self.lines[instr_index]
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

trait Decode {
    fn decode(slice_ptr: &mut &[u8]) -> Self;
}

impl Decode for u32 {
    fn decode(slice_ptr: &mut &[u8]) -> Self {
        let (val, tmp) = slice_ptr.split_at(4);
        *slice_ptr = tmp;
        let val: [u8; 4] = val.try_into().expect("slice of incorrect length.");
        return u32::from_ne_bytes(val);
    }
}

impl Decode for u8 {
    fn decode(slice_ptr: &mut &[u8]) -> Self {
        let (val, tmp) = slice_ptr.split_at(1);
        *slice_ptr = tmp;
        let val: [u8; 1] = val.try_into().expect("slice of incorrect length.");
        return u8::from_ne_bytes(val);
    }
}

impl TryFrom<Value> for Number {
    type Error = PlaceholderError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Number(num) = value {
            Ok(num)
        } else {
            Err(PlaceholderError{})
        }
    }
}

impl TryFrom<Value> for bool {
    type Error = PlaceholderError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::Boolean(val) = value {
            Ok(val)
        } else {
            Err(PlaceholderError{})
        }
    }
}

impl TryFrom<Value> for Gc<String> {
    type Error = PlaceholderError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if let Value::String(val) = value {
            Ok(val.clone())
        } else {
            Err(PlaceholderError{})
        }
    }
}

#[derive(Debug)]
pub struct PlaceholderError;
impl Display for PlaceholderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Placeholder error.")
    }
}
impl Error for PlaceholderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
