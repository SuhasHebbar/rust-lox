use crate::{interpreter::InterpreterResult, opcodes::{Chunk, ChunkIterator, Instruction, Number, Value}, precedence::ParseFn};
use std::{intrinsics::transmute, iter::{Enumerate, Peekable}, mem, ops::{Add, Sub, Mul, Div}};

const STACK_MIN_SIZE: usize = 256;

type Stack = Vec<Value>;
type Curr = Peekable<Enumerate<ChunkIterator<'static>>>;

pub struct Vm {
    chunk: Chunk,
    stack: Stack,
    instr_iter: Curr,
}

impl Vm {
    pub fn new(chunk: Chunk) -> Self {
        // https://stackoverflow.com/questions/43952104/how-can-i-store-a-chars-iterator-in-the-same-struct-as-the-string-it-is-iteratin
        // https://stackoverflow.com/questions/32300132/why-cant-i-store-a-value-and-a-reference-to-that-value-in-the-same-struct
        // This should be safe since we will not move Chunk away while using instr_iter.
        let instr_iter = unsafe { mem::transmute(chunk.instr_iter().enumerate().peekable()) };
        Vm {
            chunk,
            stack: Vec::with_capacity(STACK_MIN_SIZE),
            instr_iter,
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
        while let Some((_index, instr)) = self.instr_iter.peek() {
            #[cfg(feature = "lox_debug")]
            {
                println!("{}", self.chunk.disassemble_instruction(*_index, &instr));
            }

            match instr {
                Instruction::Return => {
                    println!("return {}", self.stack.pop().unwrap());
                    return InterpreterResult::Ok;
                }
                Instruction::Constant(cin) => {
                    let constant = self.chunk.get_value(*cin);
                    self.stack.push(constant);
                    // println!("{}", constant);
                }
                Instruction::Negate => {
                    if let Value::Number(head) = self.stack.last_mut().unwrap() {
                        *head = -*head;
                    } else {
                        self.runtime_error("Operand must be a number.");
                        return InterpreterResult::RuntimeError;
                    }
                }
                Instruction::Not => {
                    let head = self.stack.pop().unwrap();
                    let not = is_falsey(head);
                    self.stack.push(Value::Boolean(not));
                }
                Instruction::Add => {
                    self.perform_binary_op(Number::add);
                }
                Instruction::Subtract => {
                    self.perform_binary_op(Number::sub);
                }
                Instruction::Multiply => {
                    self.perform_binary_op(Number::mul);
                }
                Instruction::Divide => {
                    self.perform_binary_op(Number::div);
                }
                Instruction::Nil => self.stack.push(Value::Nil),
                Instruction::True => self.stack.push(Value::Boolean(true)),
                Instruction::False => self.stack.push(Value::Boolean(false)),
            };
            self.instr_iter.next();
        }

        return InterpreterResult::Ok;
    }

    fn runtime_error(&mut self, message: &str) {
        eprintln!("{}", message);
        let instr_index = self.instr_iter.peek().unwrap().0;
        let line_no = self.chunk.get_line(instr_index);
        eprintln!("[line {}] in script", line_no);
    }

    fn perform_binary_op(&mut self, op: impl Fn(Number, Number) -> Number) {
        if let (Value::Number(rhs), Value::Number(lhs)) = (self.peek(0), self.peek(1)) {
            let res = Value::Number(op(*lhs, *rhs));
            self.stack.pop();
            self.stack.pop();
            self.stack.push(res);
        } else {
            self.runtime_error("Operands must be numbers")
        }
    }
}

// fn peek_stk(stk: &mut Stack, distance: usize) -> &Value {
//     let stk_sz = stk.len();
//     &stk[stk_sz - 1 - distance]
// }

fn is_falsey(value: Value) -> bool {
    match value {
        Value::Nil | Value::Boolean(false) => true,
        _ => false
    }
}