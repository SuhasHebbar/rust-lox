use crate::scanner::{Scanner, TokenType as T};

pub enum InterpreterResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub struct Interpreter {}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {}
    }

    pub fn interpret(&mut self, source: &str) -> InterpreterResult {
        self.compile(source);

        InterpreterResult::Ok
    }

    fn compile(&mut self, source: &str) {
        let scanner = Scanner::new(source);
        let mut line = 0;

        for token in scanner {
            if token.line != line {
                line = token.line;
                print!("{: >4} ", line);
            } else {
                print!("   | ");
            }

            println!("{:?}", token);

            if let T::EOF = token.kind {
                break;
            }
        }

    }
}
