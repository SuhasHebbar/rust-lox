trait ByteCodeEncodeDecode: Sized {
    fn encode(&self, dest: &mut Vec<u8>);
    fn decode(src: &[u8]) -> (Self, &[u8]);
}

use std::convert::TryInto;

use lox_macros::ByteCodeEncodeDecode;

type ConstantIndex = u8;
pub type ByteCodeOffset = u16;

#[derive(Debug, ByteCodeEncodeDecode)]
pub enum Instruction {
    Return,
    LoadConstant(ConstantIndex),

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

    // Dedicated Print instruction
    Print,

    Pop,

    DefineGlobal(ConstantIndex),
    GetGlobal(ConstantIndex),
    SetGlobal(ConstantIndex),

    GetLocal(ConstantIndex),
    SetLocal(ConstantIndex),

    JumpFwdIfFalse(ByteCodeOffset),
    JumpForward(ByteCodeOffset),
    JumpBack(ByteCodeOffset)
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

impl Decode for u16 {
    fn decode(slice_ptr: &mut &[u8]) -> Self {
        let (val, tmp) = slice_ptr.split_at(2);
        *slice_ptr = tmp;
        let val: [u8; 2] = val.try_into().expect("slice of incorrect length.");
        return u16::from_ne_bytes(val);
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

