use crate::opcodes::{Chunk, Instruction};
pub struct Vm {
    chunk: Chunk,
}

enum InterpreterResult {
    Ok,
    CompileError,
    RuntimeError,
}

impl Vm {
    fn interpret(&mut self, chunk: Chunk) -> InterpreterResult {
        self.chunk = chunk;
        return self.run();
    }

    fn run(&mut self) -> InterpreterResult {
        let mut instr_iter = self.chunk.instr_iter();
        while let Some(instr) = instr_iter.next() {
            match instr {
                Instruction::Return => {
                    return InterpreterResult::Ok;
                },
                Instruction::Constant(cin) => {
                    let constant = self.chunk.get_value(cin);
                    dbg!(constant);
                    break;
                }
            };
        }

        return InterpreterResult::Ok;
    }
}
