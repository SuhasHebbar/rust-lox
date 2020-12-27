use crate::{compiler::Compiler, opcodes::Chunk, scanner::{Scanner, TokenType as T}, vm::Vm};

pub enum InterpreterResult {
    Ok,
    CompileError,
    RunttimeError,
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
    }

    fn compile(&mut self, source: &str) -> bool {
        let compiler = Compiler::new(source);
        let compiler_res = compiler.compile();
        self.chunk = compiler.chunk;

        compiler_res
    }

    fn run(&mut self) -> InterpreterResult {
        let vm = Vm::new(self.chunk);
        vm.run()
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
