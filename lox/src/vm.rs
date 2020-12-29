use crate::{heap::{Heap, LoxStr}, interpreter::{InterpreterResult, VmInit}, opcodes::{Chunk, ChunkIterator, Instruction, Number, Value}, precedence::ParseFn};
use std::{
    convert::{TryFrom, TryInto},
    intrinsics::transmute,
    iter::{Enumerate, Peekable},
    mem,
    ops::{Add, Div, Mul, Sub},
};

const STACK_MIN_SIZE: usize = 256;

type Stack = Vec<Value>;
type Curr = Peekable<Enumerate<ChunkIterator<'static>>>;

pub struct Vm {
    heap: Heap,
    chunk: Chunk,
    stack: Stack,
    instr_iter: Curr,
    had_runtime_error: bool,
}

impl Vm {
    pub fn new(vm_init: VmInit) -> Self {
        // https://stackoverflow.com/questions/43952104/how-can-i-store-a-chars-iterator-in-the-same-struct-as-the-string-it-is-iteratin
        // https://stackoverflow.com/questions/32300132/why-cant-i-store-a-value-and-a-reference-to-that-value-in-the-same-struct
        // This should be safe since we will not move Chunk away while using instr_iter.
        let VmInit {chunk, heap} = vm_init;
        let instr_iter = unsafe { mem::transmute(chunk.instr_iter().enumerate().peekable()) };
        Vm {
            heap,
            chunk,
            stack: Vec::with_capacity(STACK_MIN_SIZE),
            instr_iter,
            had_runtime_error: false,
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
                    self.stack.push(constant.clone());
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
                    let head = self.stack.last().unwrap();
                    let not = is_falsey(head);
                    self.stack.pop();
                    self.stack.push(Value::Boolean(not));
                }
                Instruction::Equal => {
                    let rhs = self.peek(0);
                    let lhs = self.peek(1);
                    let res = check_equals(lhs, rhs);
                    self.stack.pop();
                    self.stack.pop();
                    self.stack.push(Value::Boolean(res));
                }
                Instruction::Greater => {
                    self.perform_binary_op(|a: Number, b: Number| a > b);
                }
                Instruction::Less => {
                    self.perform_binary_op(|a: Number, b: Number| a < b);
                }
                Instruction::Add => {
                    self.perform_binary_op_plus();
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

            if self.had_runtime_error {
                return InterpreterResult::RuntimeError;
            }
        }

        return InterpreterResult::Ok;
    }

    fn runtime_error(&mut self, message: &str) {
        eprintln!("{}", message);
        let instr_index = self.instr_iter.peek().unwrap().0;
        let line_no = self.chunk.get_line(instr_index);
        eprintln!("[line {}] in script", line_no);
        self.had_runtime_error = true;
    }

    fn perform_binary_op_plus(& mut self)
    {
        let lhs = self.peek(1);
        let rhs = self.peek(0);

        let res: Value;

        match (lhs, rhs) {
            (Value::String(lhs), Value::String(rhs)) => {
                let lhs = lhs.as_ref();
                let rhs = rhs.as_ref();
                let mut acc: String = String::from(lhs.as_ref());
                
                acc = acc + rhs.as_ref();
                let string = LoxStr::from(acc);
                let string_ref = self.heap.intern_string(string);
                res = string_ref.into();
                dbg!("String plussing success!");
            }
            (Value::Number(lhs), Value::Number(rhs)) => {
                res = (*lhs + *rhs).into();
            }
            _ => {
                self.runtime_error("Operands must both be either numbers or strings");
                return;
            }
        }

        self.stack.pop();
        self.stack.pop();
        self.stack.push(res);
    }

    fn perform_binary_op<T, V>(&mut self, op: impl Fn(T, T) -> V)
    where
        Value: From<V>,
        T: TryFrom<Value>,
    {
        self.perform_binary_op_gen(op, "Operands must both be either numbers.");
    }

    fn perform_binary_op_gen<T, V>(&mut self, op: impl Fn(T, T) -> V, error_msg: &str)
    where
        Value: From<V>,
        T: TryFrom<Value>,
    {
        // FIXME: Using &Value here gave a mutable borrow while immutable borrow error down the line.
        // Find a way to use &Value instead of Value.
        let rhs = self.peek(0).clone().try_into();
        let lhs = self.peek(1).clone().try_into();

        let res: Value;
        if let (Ok(lhs), Ok(rhs)) = (lhs, rhs) {
            res = op(lhs, rhs).into();
            self.stack.pop();
            self.stack.pop();
            self.stack.push(res);
        } else {
            self.runtime_error(error_msg);
        }
    }
}

fn is_falsey(value: &Value) -> bool {
    match value {
        Value::Nil | Value::Boolean(false) => true,
        _ => false,
    }
}

fn check_equals(lhs: &Value, rhs: &Value) -> bool {
    if mem::discriminant(lhs) != mem::discriminant(rhs) {
        return false;
    }

    match (lhs, rhs) {
        (Value::Nil, Value::Nil) => true,
        (Value::Boolean(lhs), Value::Boolean(rhs)) => *lhs == *rhs,
        (Value::Number(lhs), Value::Number(rhs)) => *lhs == *rhs,
        (Value::String(lhs), Value::String(rhs)) => **lhs == **rhs,
        _ => panic!("unreachable"),
    }
}
