use std::{fmt::{self, Display, Formatter}, write};

use crate::{heap::{Gc, LoxStr}, opcodes::Chunk};


pub type Arity = i32;

#[derive(Debug, Clone)]
pub struct LoxFun {
    pub chunk: Chunk,
    pub name: Gc<LoxStr>,
    pub arity: Arity,
}

impl LoxFun {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            name: Gc::dangling(),
            arity: 0
        }
    }
}

impl Display for LoxFun {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<{} {}>", self.name, self.arity)
    }
}

