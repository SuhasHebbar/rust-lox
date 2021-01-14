use crate::{heap::{Gc, Heap, LoxClosure, LoxStr, Obj}, interpreter::{InterpreterResult, VmInit}, native::{ClockNative, LoxNativeFun, ValueToStrConverter}, object::{FunctionType, LoxFun}, opcodes::{ArgCount, Chunk, ChunkIterator, ConstantIndex, Instruction, Number, Value}};
use std::time;
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
    iter::{Peekable},
    mem,
    ops::{Add, Div, Mul, Sub},
};

const FRAMES_MIN_SIZE: usize = 64;
const STACK_MIN_SIZE: usize = FRAMES_MIN_SIZE * (StackIndex::MAX as usize + 1);

pub type StackIndex = u8;
pub type FrameIndex = usize;

type Stack = Vec<Value>;
type Curr = Peekable<ChunkIterator<'static>>;
type Globals = HashMap<Gc<LoxStr>, Value>;

pub struct Vm {
    heap: Heap,
    stack: Stack,
    call_frames: Vec<CallFrame>,
    globals: Globals,
    had_runtime_error: bool,
}

impl Vm {
    pub fn new(vm_init: VmInit) -> Self {
        // https://stackoverflow.com/questions/43952104/how-can-i-store-a-chars-iterator-in-the-same-struct-as-the-string-it-is-iteratin
        // https://stackoverflow.com/questions/32300132/why-cant-i-store-a-value-and-a-reference-to-that-value-in-the-same-struct
        // This should be safe since we will not move any Chunks away while using instr_iter.
        let VmInit { function, heap } = vm_init;
        let mut globals = HashMap::new();

        let mut stack = Vec::with_capacity(STACK_MIN_SIZE); 
        stack.push(Value::Function(function));

        initialize_built_ins(&heap, &mut globals);

        let instr_iter = get_cursor(function.chunk.instr_iter());

        let mut call_frames = Vec::with_capacity(FRAMES_MIN_SIZE);
        call_frames.push(CallFrame { function, ip: instr_iter, frame_index: 0});

        Vm {
            heap,
            stack,
            call_frames,
            globals,
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
        // transmute is used here as the callframe reference will cause issues with methods that
        // borrow self down the line.
        // The Vm struct methods need to be refactored to not need this usage.
        let mut call_frame: &mut CallFrame = get_callframe(&mut self.call_frames);

        #[cfg(feature = "lox_debug")]
        {
            println!("{}", call_frame.get_chunk());
            // println!("Starting Execution");
        }


        while let Some((index, instr)) = call_frame.ip.peek() {
            let instr = *instr;
            let index = *index;

            #[cfg(feature = "lox_debug")]
            {
                println!("{}", call_frame.get_chunk().disassemble_instruction(index, &instr));
            }

            match instr {
                Instruction::Return => {
                    let result = self.stack.pop().unwrap();
                    let result_slot = call_frame.frame_index;

                    drop(call_frame);
                    self.call_frames.pop();

                    if self.call_frames.len() == 0 {
                        self.stack.pop();
                        return InterpreterResult::Ok;
                    }

                    self.stack.truncate(result_slot);
                    self.stack.push(result);

                    call_frame = get_callframe(&mut self.call_frames);






                }
                Instruction::LoadConstant(cin) => {
                    let constant = call_frame.get_value(cin);
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
                    let rhs = self.stack.peek(0);
                    let lhs = self.stack.peek(1);
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
                Instruction::Print => {
                    let top = self.stack.pop().unwrap();
                    println!("{}", top);
                }
                Instruction::Pop => {
                    self.stack.pop();
                }
                Instruction::DefineGlobal(var_index) => {
                    let var_name: Gc<LoxStr> = call_frame.get_value(var_index).try_into().unwrap();
                    let value = self.stack.pop().unwrap();
                    self.globals.insert(var_name, value);
                }
                Instruction::SetGlobal(var_index) => {
                    let var_name: Gc<LoxStr> = call_frame.get_value(var_index).try_into().unwrap();
                    let value = self.stack.peek(0).clone();
                    if let None = self.globals.insert(var_name.clone(), value) {
                        self.globals.remove(&var_name);
                        self.runtime_error(format!("Undefined variable '{}'.", var_name));
                        return InterpreterResult::RuntimeError;
                    }
                }
                Instruction::GetGlobal(var_index) => {
                    let var_name: Gc<LoxStr> = call_frame.get_value(var_index).try_into().unwrap();
                    if let Some(value) = self.globals.get(&var_name) {
                        self.stack.push(value.clone());
                    } else {
                        self.runtime_error(format!("Undefined variable '{}'.", var_name));
                    }
                }
                Instruction::GetLocal(var_index) => {
                    self.stack.push(self.stack[call_frame.frame_index + var_index as usize]);
                }
                Instruction::SetLocal(var_index) => {
                    let var_index = var_index;

                    self.stack[call_frame.frame_index + var_index as usize] = *self.stack.peek(0);
                }
                Instruction::JumpFwdIfFalse(offset) => {
                    let jump_index = index + offset as usize;
                    let stack_val = self.stack.peek(0);
                    if is_falsey(stack_val) {
                        call_frame.ip =
                            get_cursor(call_frame.get_chunk().instr_iter_jump(jump_index));
                        continue;
                    }
                }
                Instruction::JumpForward(offset) => {
                    let jump_index = index + offset as usize;
                    call_frame.ip = get_cursor(call_frame.get_chunk().instr_iter_jump(jump_index));
                    continue;
                }
                Instruction::JumpBack(offset) => {
                    let jump_index = index - offset as usize;
                    call_frame.ip = get_cursor(call_frame.get_chunk().instr_iter_jump(jump_index));
                    continue;
                }
                Instruction::Call(arg_count) => {
                    drop(call_frame);
                    let callee = *self.peek(arg_count as usize);
                    if !self.call_value(callee, arg_count) {
                        return InterpreterResult::RuntimeError;
                    }

                    call_frame = get_callframe(&mut self.call_frames);
                    continue;
                }
                Instruction::Closure(func_index) => {
                    if let Value::Function(function) = call_frame.get_value(func_index) {
                        let closure = self.heap.manage(LoxClosure::new(function.clone()));
                        self.stack.push(Value::Closure(closure));
                    } else {
                        panic!("Non closure value loaded for Closure opcode");
                    }
                }
            };
            call_frame.ip.next();

            if self.had_runtime_error {
                return InterpreterResult::RuntimeError;
            }
        }


        return InterpreterResult::Ok;
    }


    fn call_value(&mut self, callee: Value, arg_count: ArgCount) -> bool {
        match callee {
            Value::Function(fun_ptr) => {
                self.call(fun_ptr, arg_count)
            }
            Value::NativeFunction(mut fun_ptr) => {
                let frame_index = self.stack.len() - arg_count as usize;
                let stack_window = &self.stack[frame_index..];
                let res = fun_ptr.callable.call(arg_count, stack_window, &self.heap);
                self.stack.truncate(frame_index - 1);
                self.stack.push(res);

                // Since we skip ip.next after calls we need to add call ip.next for native calls ourselves.
                self.call_frames.last_mut().unwrap().ip.next();
                true
            }
            _ => {
                self.runtime_error("Can only call functions and classes.");
                false
            }
        }
    }

    fn call(&mut self, fun_ptr: Gc<LoxFun>, arg_count: ArgCount) -> bool {
        if arg_count as i32 != fun_ptr.arity {
            self.runtime_error(format!("Expected {} arguments but got {}.", fun_ptr.arity, arg_count));
            return false;
        }
        let cursor = get_cursor(fun_ptr.chunk.instr_iter());
        let call_frame = CallFrame { function: fun_ptr, ip: cursor, frame_index: self.stack.len() - arg_count as usize - 1};

        if self.call_frames.len() == FRAMES_MIN_SIZE {
            self.runtime_error("Stack overflow.");
            return false;
        }
        self.call_frames.push(call_frame);
        true
    }


    fn runtime_error(&mut self, message: impl AsRef<str>) {
        // start moving out functions from borrowing self.
        runtime_error(&mut self.call_frames, &mut self.had_runtime_error, message);
        self.call_frames.truncate(1);
    }

    fn perform_binary_op_plus(&mut self) {
        let lhs = self.stack.peek(1);
        let rhs = self.stack.peek(0);

        let res: Value;

        match (lhs, rhs) {
            (Value::String(lhs), Value::String(rhs)) => {
                let mut acc = lhs.to_string();

                acc = acc + rhs.as_str();
                let string_ref = self.heap.intern_string(acc);
                res = string_ref.into();
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
        for<'a> T: TryFrom<&'a Value>,
        T: Copy,
    {
        self.perform_binary_op_gen(op, "Operands must both be either numbers.");
    }

    fn perform_binary_op_gen<T, V>(&mut self, op: impl Fn(T, T) -> V, error_msg: &str)
    where
        Value: From<V>,
        for<'a> T: TryFrom<&'a Value>,
        T: Copy,
    {
        let lhs = self.stack.peek(1).try_into();
        let rhs = self.stack.peek(0).try_into();

        let temp = (lhs, rhs);
        match &temp {
            (Ok(ref lhs), Ok(ref rhs)) => {
                let res = op(*lhs, *rhs).into();
                drop(temp);
                self.stack.pop();
                self.stack.pop();
                self.stack.push(res);
            }
            _ => {
                drop(temp);
                self.runtime_error(error_msg);
            }
        }
    }
}

fn initialize_built_ins(heap: &Heap, globals: &mut Globals) {
    let clock_native = LoxNativeFun::new(ClockNative::new());
    let value_to_str = LoxNativeFun::new(ValueToStrConverter::new());

    let clock_native = Value::NativeFunction(heap.manage(clock_native));
    let value_to_str = Value::NativeFunction(heap.manage(value_to_str));

    let clock_native_name = heap.intern_string("clock");
    let value_to_str_name = heap.intern_string("str");

    globals.insert(clock_native_name, clock_native);
    globals.insert(value_to_str_name, value_to_str);

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

fn get_cursor(chunk_iter: ChunkIterator) -> Curr {
    unsafe { mem::transmute(chunk_iter.peekable()) }
}

struct Functions {
    current: Gc<LoxFun>,
    list: Vec<Obj<LoxFun>>,
    type_: FunctionType,
}

struct CallFrame {
    function: Gc<LoxFun>,
    ip: Curr,
    frame_index: FrameIndex
}

impl CallFrame {
    fn get_chunk(&self) -> &Chunk {
        &self.function.chunk
    }

    fn get_value(&self, index: ConstantIndex) -> &Value {
        self.get_chunk().get_value(index)
    }
}

trait PeekFromTop {
    type Target;
    fn peek(&self, distance: usize) -> &Self::Target;
}

impl PeekFromTop for Vec<Value> {
    type Target = Value;

    fn peek(&self, distance: usize) -> &Value {

        let stk_sz = self.len();
        &self[stk_sz - 1 - distance]
    }
}

fn runtime_error(call_frames: &mut Vec<CallFrame>, had_runtime_error: &mut bool, message: impl AsRef<str>) {
    let message = message.as_ref();
    eprintln!("{}", message);
    for call_frame in call_frames.iter_mut().rev() {
        let instr_index = call_frame.ip.peek().unwrap().0;
        let line_no = call_frame.get_chunk().get_line(instr_index);
        let fun_name = if (*call_frame.function.name).as_ref() == "" {
            "script"
        } else {
            &call_frame.function.name
        };

        eprintln!("[line {}] in {}", line_no, fun_name);
    }

    *had_runtime_error = true;
}

fn get_callframe(call_frames: &mut Vec<CallFrame>) -> &'static mut CallFrame {
unsafe { mem::transmute(call_frames.last_mut().unwrap()) }
}