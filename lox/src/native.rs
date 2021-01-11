use std::{fmt::{self, Formatter}, time::Instant, write};

use fmt::Display;

use crate::{heap::Heap, opcodes::{ArgCount, Value}};

// pub fn clock_native(arg_count: ArgCount, args: &[Value]) -> Value {
//     Value::Number(program_start.elapsed().as_secs_f64())
// }

pub trait NativeFun: fmt::Debug + 'static {
    fn call(&mut self, arg_count: ArgCount, args: &[Value], heap: &Heap) -> Value;
}

#[derive(Debug)]
pub struct LoxNativeFun {
    pub callable: Box<dyn NativeFun>,
}

impl Display for LoxNativeFun {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<native fn>")
    }
}

impl LoxNativeFun {
    pub fn new(callable: impl NativeFun) -> Self {
        Self {
            callable: Box::new(callable)
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClockNative {
    start: Instant
}

impl ClockNative {
    pub fn new() -> Self {
        Self {
            start: Instant::now()
        }
    }
}

impl NativeFun for ClockNative {
    fn call(&mut self, _arg_count: ArgCount, _args: &[Value], _heap: &Heap) -> Value {
        Value::Number(self.start.elapsed().as_secs_f64())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ValueToStrConverter {}

impl ValueToStrConverter {
    pub fn new() -> Self {
        Self {}
    }
}

impl NativeFun for ValueToStrConverter {
    fn call(&mut self, arg_count: ArgCount, args: &[Value], heap: &Heap) -> Value {
        if arg_count < 1 {
            let str_ref = heap.intern_string("");
            Value::String(str_ref)
        } else {
            let str_ref = heap.intern_string(args[0].to_string());
            Value::String(str_ref)
        }
    }
}