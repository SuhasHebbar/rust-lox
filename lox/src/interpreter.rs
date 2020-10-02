use crate::{opcodes::Chunk, scanner::{Scanner, TokenType as T}};

pub enum InterpreterResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub struct Interpreter {
    chunk: Chunk
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            chunk: Chunk::new()
        }
    }

    pub fn interpret(&mut self, source: &str) -> InterpreterResult {
        if !self.compile(source) {
            return InterpreterResult::CompileError;
        }

        return self.run();

        InterpreterResult::Ok
    }

    fn compile(&mut self, source: &str) -> bool {
        let scanner = Scanner::new(source);
//         advance();
//   expression();
//   consume(TOKEN_EOF, "Expect end of expression.");
        todo!()
    }

    fn run(&mut self) -> InterpreterResult {
        todo!()
    }

    fn print_tokens(&mut self, source: &str) {
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
