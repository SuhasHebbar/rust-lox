#![feature(prelude_import)]
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;
trait ByteCodeEncodeDecode: Sized {
    fn encode(&self, dest: &mut Vec<u8>);
    fn decode(src: &[u8]) -> (Self, &[u8]);
}
use std::convert::TryInto;
use lox_macros::ByteCodeEncodeDecode;
type ConstantIndex = u32;
enum ByteCode {
    A,
    B(u32),
    C(ConstantIndex),
}
impl ByteCodeEncodeDecode for ByteCode {
    fn encode(&self, dest: &mut Vec<u8>) {
        match self {
            ByteCode::A => {
                dest.push(0usize as u8);
            }
            ByteCode::B(a0) => {
                dest.push(1usize as u8);
                dest.extend_from_slice(&a0.to_ne_bytes()[..]);
            }
            ByteCode::C(a0) => {
                dest.push(2usize as u8);
                dest.extend_from_slice(&a0.to_ne_bytes()[..]);
            }
        };
    }
    fn decode(src: &[u8]) -> (Self, &[u8]) {
        let mut slice_ptr = &src[1..];
        let byte = src[0];
        match byte as usize {
            0usize => (ByteCode::A, slice_ptr),
            1usize => {
                let a0 = u32::decode(&mut slice_ptr);
                (ByteCode::B(a0), slice_ptr)
            }
            2usize => {
                let a0 = ConstantIndex::decode(&mut slice_ptr);
                (ByteCode::C(a0), slice_ptr)
            }
            _ => ::std::rt::begin_panic("Invalid instruction byte code"),
        }
    }
}
extern crate test;
#[cfg(test)]
#[rustc_test_marker]
pub const get_ast: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("get_ast"),
        ignore: false,
        allow_fail: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(|| test::assert_test_result(get_ast())),
};
fn get_ast() {
    let a = ByteCode::A;
    {
        ::std::io::_print(::core::fmt::Arguments::new_v1(
            &["damnn\n"],
            &match () {
                () => [],
            },
        ));
    };
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
#[main]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&get_ast])
}
