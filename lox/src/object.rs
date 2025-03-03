use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{self, Display, Formatter},
    mem,
    ptr::NonNull,
    write,
};

use crate::{
    heap::{Gc, LoxStr, Trace},
    native::LoxNativeFun,
    opcodes::{Chunk, Value},
    vm::StackIndex,
};

pub type Arity = i32;

#[derive(Debug, Clone)]
pub struct LoxFun {
    pub chunk: Chunk,
    pub name: Gc<LoxStr>,
    pub arity: Arity,
    pub upvalues: Box<[UpvalueSim]>,
}

impl LoxFun {
    pub fn new(name: Gc<LoxStr>) -> Self {
        Self {
            chunk: Chunk::new(),
            name,
            arity: 0,
            upvalues: Box::new([]),
        }
    }
}

impl Default for LoxFun {
    fn default() -> Self {
        Self {
            chunk: Chunk::new(),
            name: Gc::dangling(),
            arity: 0,
            upvalues: Box::new([]),
        }
    }
}

impl Trace for LoxFun {
    fn trace(&self, grey_stack: &mut crate::heap::GreyStack) {
        self.name.mark_if_needed(grey_stack);
        self.chunk.mark_if_needed(grey_stack);
    }

    fn bytes_allocated(&self) -> usize {
        let self_size = mem::size_of::<Self>();
        let upvalues_size = mem::size_of_val(self.upvalues.as_ref());

        self_size + upvalues_size
    }
}

impl Display for LoxFun {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<{} {}>", self.name, self.arity)
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum FunctionType {
    Function,
    Method,
    Initializer,
    Script,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpvalueSim {
    Local(StackIndex),
    Upvalue(StackIndex),
}

#[derive(Debug, Clone)]
pub struct Upvalue {
    pub location: NonNull<Value>,
    value: Value,
}

impl Upvalue {
    pub fn new(ptr: *mut Value) -> Self {
        let location = unsafe { NonNull::new_unchecked(ptr) };

        Self {
            location,
            value: Value::Nil,
        }
    }

    pub fn close(&mut self) {
        unsafe {
            self.value = *self.location.as_ref();
            self.location = NonNull::new_unchecked(&mut self.value as *mut _);
        }
    }

    pub fn value_ptr(&self) -> *mut Value {
        self.location.as_ptr()
    }
}

impl Trace for Upvalue {
    fn trace(&self, grey_stack: &mut crate::heap::GreyStack) {
        self.as_ref().mark_if_needed(grey_stack);
    }

    fn bytes_allocated(&self) -> usize {
        mem::size_of::<Self>()
    }
}

impl AsRef<Value> for Upvalue {
    fn as_ref(&self) -> &Value {
        unsafe { self.location.as_ref() }
    }
}

impl AsMut<Value> for Upvalue {
    fn as_mut(&mut self) -> &mut Value {
        unsafe { self.location.as_mut() }
    }
}

#[derive(Debug, Clone)]
pub struct LoxClosure {
    pub function: Gc<LoxFun>,
    pub upvalues: Box<[Gc<Upvalue>]>,
}

impl LoxClosure {
    pub fn new(function: Gc<LoxFun>) -> Self {
        Self {
            function,
            upvalues: Box::new([]),
        }
    }
}

impl Trace for LoxClosure {
    fn trace(&self, grey_stack: &mut crate::heap::GreyStack) {
        self.function.mark_if_needed(grey_stack);
        for upvalue in self.upvalues.iter() {
            upvalue.mark_if_needed(grey_stack);


        }
    }

    fn bytes_allocated(&self) -> usize {
        let self_size = mem::size_of::<Self>();
        let slice_size = mem::size_of_val(self.upvalues.as_ref());

        self_size + slice_size
    }
}

#[derive(Debug)]
pub struct LoxClass {
    name: Gc<LoxStr>,
    pub methods: Fields
}

impl LoxClass {
    pub fn new(name: Gc<LoxStr>) -> Self {
        Self { name , methods: HashMap::new()}
    }
}

impl Trace for LoxClass {
    fn trace(&self, grey_stack: &mut crate::heap::GreyStack) {
        self.name.mark_if_needed(grey_stack);

        for (k, v) in self.methods.iter() {
            k.mark_if_needed(grey_stack);
            v.mark_if_needed(grey_stack);
        }
    }

    fn bytes_allocated(&self) -> usize {
        let methods_heap_size =
            self.methods.capacity() * (mem::size_of::<Value>() + mem::size_of::<Gc<LoxStr>>());

        methods_heap_size + mem::size_of::<Self>()
    }
}

pub type Fields = HashMap<Gc<LoxStr>, Value>;

#[derive(Debug)]
pub struct LoxInstance {
    pub class: Gc<LoxClass>,
    pub fields: Fields,
}

impl LoxInstance {
    pub fn new (class: Gc<LoxClass>) -> Self {
        Self {
            class,
            fields: HashMap::new(),
        }
    }
}

impl Trace for LoxInstance {
    fn trace(&self, grey_stack: &mut crate::heap::GreyStack) {
        self.class.mark_if_needed(grey_stack);
        for (k, v) in self.fields.iter() {
            k.mark_if_needed(grey_stack);
            v.mark_if_needed(grey_stack);
        }
    }

    fn bytes_allocated(&self) -> usize {
        let self_size = mem::size_of::<Self>();
        let fields_heap_size =
            self.fields.capacity() * (mem::size_of::<Value>() + mem::size_of::<Gc<LoxStr>>());

        self_size + fields_heap_size
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LoxBoundMethod {
    pub method: Gc<LoxClosure>,
    pub receiver: Value
}

impl LoxBoundMethod {
    /// Makes the assumption that the provided receiver is a LoxInstance.
    pub fn new(method: Gc<LoxClosure>, receiver: Value) -> Self {
        Self {
            method,
            receiver
        }
    }
}

impl Trace for LoxBoundMethod {
    fn trace(&self, grey_stack: &mut crate::heap::GreyStack) {
        self.method.mark_if_needed(grey_stack);
        self.receiver.mark_if_needed(grey_stack);
    }

    fn bytes_allocated(&self) -> usize {
        mem::size_of::<Self>()
    }
}