trait ByteCodeEncodeDecode: Sized {
    fn encode(&self, dest: &mut Vec<u8>);
    fn decode(src: &[u8]) -> (Self, &[u8]);
}

use std::convert::TryInto;

use lox_macros::ByteCodeEncodeDecode;

type ConstantIndex = u32;

#[derive(ByteCodeEncodeDecode)]
enum ByteCode {
    A,
    B(u32),
    C(ConstantIndex),
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
