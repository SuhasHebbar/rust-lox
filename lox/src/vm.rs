use crate::{interpreter::InterpreterResult, opcodes::{Chunk, Instruction, Value, Number}};
use std::ops::{Add, Sub, Mul, Div};

const STACK_MIN_SIZE: usize = 256;

pub struct Vm {
    chunk: Chunk,
    stack: Vec<Value>,
}

impl Vm {
    pub fn new(chunk: Chunk) -> Self {
        Vm {
            chunk,
            stack: Vec::with_capacity(STACK_MIN_SIZE),
        }
    }

    pub fn clear_stack(&mut self) {
        self.stack.clear();
    }

    pub fn run(&mut self) -> InterpreterResult {
        let mut instr_iter = self.chunk.instr_iter().enumerate();
        while let Some((index, instr)) = instr_iter.next() {
            #[cfg(feature = "lox_debug")]
            {
                println!("{}", self.chunk.disassemble_instruction(index, &instr));
            }

            match instr {
                Instruction::Return => {
                    
                    println!("return {}", self.stack.pop().unwrap());
                    return InterpreterResult::Ok;
                }
                Instruction::Constant(cin) => {
                    let constant = self.chunk.get_value(cin);
                    self.stack.push(constant);
                    println!("{}", constant);
                },
                Instruction::Negate => {
                    let Value::Number(head) = self.stack.last_mut().unwrap();
                    *head = -*head;

                },
                Instruction::Add => {
                    perform_binary_op(&mut self.stack, Number::add);
                },
                Instruction::Subtract => {
                    perform_binary_op(&mut self.stack, Number::sub);
                },
                Instruction::Multiply => {
                    perform_binary_op(&mut self.stack, Number::mul);
                },
                Instruction::Divide => {
                    perform_binary_op(&mut self.stack, Number::div);
                }
            };
        }

        return InterpreterResult::Ok;
    }

}

fn perform_binary_op(stack: &mut Vec<Value>, op: impl Fn(Number, Number) -> Number) {
    let (Value::Number(rhs), Value::Number(lhs)) = (stack.pop().unwrap(), stack.pop().unwrap());
    stack.push(Value::Number(op(lhs, rhs)));
}