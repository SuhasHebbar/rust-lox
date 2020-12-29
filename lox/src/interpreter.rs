use crate::{
    compiler::Compiler,
    heap::Heap,
    opcodes::Chunk,
    scanner::{Scanner, TokenType as T},
    vm::Vm,
};

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
        let compile_res = self.compile(source);
        if let Some(vm_init) = compile_res {
            return self.run(vm_init);
        } else {
            return InterpreterResult::CompileError;
        }
    }

    fn compile(&mut self, source: &str) -> Option<VmInit> {
        let mut compiler = Compiler::new(source);
        let compiler_res = compiler.compile();
        let chunk = compiler.chunk;
        let heap = compiler.heap;

        if compiler.had_error {
            None
        } else {
            Some(VmInit { chunk, heap })
        }
    }

    fn run(&mut self, vm_init: VmInit) -> InterpreterResult {
        let mut vm = Vm::new(vm_init);
        vm.run()
    }

    #[allow(dead_code)]
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

pub struct VmInit {
    pub chunk: Chunk,
    pub heap: Heap,
}
