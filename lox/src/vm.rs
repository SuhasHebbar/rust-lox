use crate::{
    interpreter::InterpreterResult,
    opcodes::{Chunk, Instruction, Number, Value},
};
use std::ops::{Add, Div, Mul, Sub};

const STACK_MIN_SIZE: usize = 256;

type Stack = Vec<Value>;

pub struct Vm {
    chunk: Chunk,
    stack: Stack,
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

    fn peek(&self, distance: usize) -> &Value {
        let stk_sz = self.stack.len();
        &self.stack[stk_sz - 1 - distance]
    }

    pub fn run(&mut self) -> InterpreterResult {
        let mut instr_iter = self.chunk.instr_iter().enumerate();
        while let Some((index, instr)) = instr_iter.next() {
            #[cfg(feature = "lox_debug")]
            {
                println!("{}", self.chunk.disassemble_instruction(index, &instr));
            }

            let stk_ref = &mut self.stack;
            let runtime_error_handle = |a, b| self.runtime_error(a, b);

            match instr {
                Instruction::Return => {
                    println!("return {}", self.stack.pop().unwrap());
                    return InterpreterResult::Ok;
                }
                Instruction::Constant(cin) => {
                    let constant = self.chunk.get_value(cin);
                    self.stack.push(constant);
                    // println!("{}", constant);
                }
                Instruction::Negate => {
                    if let Value::Number(head) = self.stack.last_mut().unwrap() {
                        *head = -*head;
                    } else {
                        self.runtime_error("Operand must be a number.", index);
                        return InterpreterResult::RuntimeError;
                    }
                }
                Instruction::Add => {
                    perform_binary_op(stk_ref, index, Number::add, runtime_error_handle);
                }
                Instruction::Subtract => {
                    perform_binary_op(stk_ref, index, Number::sub, runtime_error_handle);
                }
                Instruction::Multiply => {
                    perform_binary_op(stk_ref, index, Number::mul, runtime_error_handle);
                }
                Instruction::Divide => {
                    perform_binary_op(stk_ref, index, Number::div, runtime_error_handle);
                }
            };
        }

        return InterpreterResult::Ok;
    }

    fn runtime_error(&self, message: &str, instr_index: usize) {
        eprintln!("{}", message);
        let line_no = self.chunk.get_line(instr_index);
        eprintln!("[line {}] in script", line_no);
    }

    fn perform_binary_op(
        &mut self
        op: impl Fn(Number, Number) -> Number,
    ) {
        if let (Value::Number(rhs), Value::Number(lhs)) = (self.peek(0), self.peek( 1)) {
            let res = Value::Number(op(*lhs, *rhs));
            self.stack.pop();
            self.stack.pop();
            self.stack.push(res);
        } else {
            // runtime_error_handle("Operands must be numbers", instr_index)
        }
    }
}

// fn peek_stk(stk: &mut Stack, distance: usize) -> &Value {
//     let stk_sz = stk.len();
//     &stk[stk_sz - 1 - distance]
// }