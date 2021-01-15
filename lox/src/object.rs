use std::{borrow::Cow, fmt::{self, Display, Formatter}, ptr::NonNull, write};

use crate::{heap::{Gc, LoxStr}, opcodes::{Chunk, Value}, vm::StackIndex};


pub type Arity = i32;

#[derive(Debug, Clone)]
pub struct LoxFun {
    pub chunk: Chunk,
    pub name: Gc<LoxStr>,
    pub arity: Arity,
    pub upvalues: Box<[UpvalueSim]>,
}

impl LoxFun {
    pub fn new(name: Gc<LoxStr>) -> Self {
        Self {
            chunk: Chunk::new(),
            name,
            arity: 0,
            upvalues: Box::new([])
        }
    }
}

impl Default for LoxFun {
    fn default() -> Self {
        Self {
            chunk: Chunk::new(),
            name: Gc::dangling(),
            arity: 0,
            upvalues: Box::new([])
        }
    }
}

impl Display for LoxFun {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<{} {}>", self.name, self.arity)
    }
}

#[derive(Debug)]
pub enum FunctionType {
    Function,
    Script,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpvalueSim {
    Local(StackIndex),
    Upvalue(StackIndex),
}

#[derive(Debug, Clone)]
pub struct Upvalue {
    pub location: NonNull<Value>,
    value: Value
}

impl Upvalue {
    pub fn new(ptr: *mut Value) -> Self {
        let location = unsafe { NonNull::new_unchecked(ptr) };

        Self {
            location,
            value: Value::Nil
        }
    }

    pub fn close(&mut self) {
        unsafe {
            self.value = *self.location.as_ref();
            self.location = NonNull::new_unchecked(&mut self.value as *mut _);
        }
    }

    pub fn value_ptr(&self) -> *mut Value {
        self.location.as_ptr()
    }
}

// impl AsPtr

impl AsRef<Value> for Upvalue {
    fn as_ref(&self) -> &Value {
        unsafe {
            self.location.as_ref()
        }
    }
}

impl AsMut<Value> for Upvalue {
    fn as_mut(&mut self) -> &mut Value {
        unsafe {
            self.location.as_mut()
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoxClosure {
    pub function: Gc<LoxFun>,
    pub upvalues: Vec<Gc<Upvalue>>,
}

impl LoxClosure {
    pub fn new(function: Gc<LoxFun>) -> Self {
        Self {
            function,
            upvalues: Vec::with_capacity(function.upvalues.len()),
        }
    }
}