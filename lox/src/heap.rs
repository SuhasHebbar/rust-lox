/// Currently this is just the bare beginnings of a scaffold for the lox GC.
use std::{borrow::{Borrow, BorrowMut}, cell::RefCell, collections::{HashMap, HashSet}, fmt::{self, Display, Formatter}, hash::Hasher, ops::{Deref, DerefMut}, ptr::NonNull, rc::Rc};
use std::{hash::Hash, mem};

use crate::{object, vm::Vm};

pub type GreyStack = Vec<&'static dyn Trace>;

pub struct Heap {
    interned_strs: RefCell<HashMap<&'static LoxStr, Box<Obj<LoxStr>>>>,
    objects: RefCell<Vec<Box<dyn HeapObj>>>,
    grey_stack: RefCell<GreyStack>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            interned_strs: RefCell::new(HashMap::new()),
            objects: RefCell::new(Vec::new()),
            grey_stack: RefCell::new(Vec::new())
        }
    }

    fn collect_garbage_if_needed(&self, vm: &Vm) {
        #[cfg(feature = "debug_log_gc")]
        println!("-- gc begin");

        todo!();

        #[cfg(feature = "debug_log_gc")]
        println!("-- gc end");
    }

    fn mark_roots(&self, vm: &Vm) {
        let mut grey_stack_borrow = self.grey_stack.borrow_mut();
        let grey_stack = grey_stack_borrow.as_mut();

        for call_frame in vm.call_frames.iter() {
            call_frame.closure.mark_if_needed(grey_stack);
        }

        for upvalue in vm.open_upvalues.iter() {
            upvalue.mark_if_needed(grey_stack);
        }

        for value in vm.stack.iter() {
            value.mark_if_needed(grey_stack);
        }

        for (key, value) in vm.globals.iter() {
            key.mark_if_needed(grey_stack);
            value.mark_if_needed(grey_stack);
        }

    }

    fn mark_heap(&self, vm: &Vm) {
        self.mark_roots(vm);

        let mut grey_stack_borrow = self.grey_stack.borrow_mut();
        let grey_stack: &mut GreyStack = grey_stack_borrow.as_mut();

        while grey_stack.len() > 0 {
            let marked = grey_stack.pop().unwrap();
            marked.trace(grey_stack);
        }
    }

    fn sweep_heap(&self) {
        let mut objects = self.objects.borrow_mut();

        objects.retain(|heap_obj| heap_obj.is_marked());

        for object in objects.iter_mut() {
            object.unmark();
        }

        let mut interned_strs = self.interned_strs.borrow_mut();

        interned_strs.retain(|k, v| v.is_marked());

        for (k, v) in interned_strs.iter_mut() {
            v.unmark();
        }
    }

    pub fn manage_gc<T: Trace>(&self, value: T, vm: &Vm) -> Gc<T> {
        self.collect_garbage_if_needed(vm);
        self.manage(value)
    }

    pub fn intern_string_gc(&self, str_ref: impl AsRef<str>, vm: &Vm) -> Gc<LoxStr> {
        self.collect_garbage_if_needed(vm);
        self.intern_string(str_ref)
    }

    pub fn manage<T: Trace>(&self, value: T) -> Gc<T> {
        let mut boxed = Box::new(Obj::new(value));
        let ptr = boxed.as_mut() as *mut _;

        #[cfg(feature = "debug_log_gc")]
        println!(
            "Allocate {:?}, size = {}, type = {}",
            ptr,
            std::mem::size_of_val(boxed.as_ref()),
            std::any::type_name::<T>()
        );

        self.objects.borrow_mut().push(boxed);
        Gc::from(ptr)
    }

    pub fn intern_string(&self, str_ref: impl AsRef<str>) -> Gc<LoxStr> {
        // FIXME: This LoxStr may be discarded if it already exists in the intern cache.
        // To create this we clone the input ref hence potentially allocating uncessarily.
        // Need to clone only when necessary.
        let string = LoxStr::from(str_ref.as_ref());
        self.intern_string_internal(string)
    }

    // This is separated from intern_string to avoid Generic impl duplication.
    fn intern_string_internal(&self, string: LoxStr) -> Gc<LoxStr> {
        let mut interned_strs = self.interned_strs.borrow_mut();
        let heapobj = interned_strs.get_mut(&string);

        let obj_ptr;

        if let Some(heapobj) = heapobj {
            obj_ptr = heapobj.as_mut() as *mut Obj<LoxStr>;
        } else {
            drop(heapobj);
            drop(interned_strs);
            let mut value = Box::new(Obj::new(string));
            obj_ptr = value.as_mut() as *mut Obj<LoxStr>;
            let new_key = unsafe { mem::transmute(&value.data) };
            self.interned_strs.borrow_mut().insert(new_key, value);
        }
        Gc::from(obj_ptr)
    }
}

#[derive(Clone, Debug)]
pub struct Obj<T: ?Sized + 'static> {
    marked: bool,
    data: T,
}

impl<T> HeapObj for Obj<T> {
    fn is_marked(&self) -> bool {
        self.marked
    }

    fn mark(&mut self) {
        self.marked = true;
    }

    fn unmark(&mut self) {
        self.marked = false;
    }
}

impl<T> Obj<T> {
    pub fn new(data: T) -> Self {
        Self {
            marked: false,
            data,
        }
    }
}

impl<T> Hash for Obj<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Gc<T: 'static + Trace> {
    ptr: *mut Obj<T>,
}

impl<T: Trace> Copy for Gc<T> {}
impl<T: Trace> Clone for Gc<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Trace> Gc<T> {
    pub fn dangling() -> Self {
        Self {
            ptr: 0 as *mut Obj<T>,
        }
    }

    pub fn is_marked(&self) -> bool {
        self.as_obj().is_marked()
    }

    pub fn mark(&self) {
        self.as_obj_mut().mark();
    }

    pub fn mark_if_needed(&self, grey_stack: &mut GreyStack) {
        if !self.is_marked() {
            self.mark();
            grey_stack.push(self.get_ref());
        }
    }

    pub fn unmark(&self) {
        self.as_obj_mut().unmark();
    }

    pub fn as_obj(&self) -> &'static Obj<T> {
        unsafe { &*self.ptr }
    }

    pub fn as_obj_mut(&self) -> &'static mut Obj<T> {
        unsafe { &mut *self.ptr }
    }

    pub fn get_ref(&self) -> &'static T {
        &self.as_obj().data
    }
}

impl<T: Trace> From<*mut Obj<T>> for Gc<T> {
    fn from(val: *mut Obj<T>) -> Self {
        Self { ptr: val }
    }
}

impl<T: Trace> Deref for Gc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.as_obj().data
    }
}

impl<T: Trace> DerefMut for Gc<T> {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.as_obj_mut().data
    }
}

impl<T: Trace> Borrow<T> for Gc<T> {
    fn borrow(&self) -> &T {
        &self.as_obj().data
    }
}

impl<T: Trace> BorrowMut<T> for Gc<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.as_obj_mut().data
    }
}

impl<T: Trace> AsRef<T> for Gc<T> {
    fn as_ref(&self) -> &T {
        &self.as_obj().data
    }
}

impl<T: Trace> AsMut<T> for Gc<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.as_obj_mut().data
    }
}

impl<T> Display for Gc<T>
where
    T: Display + Trace,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

// Adding wrapper since this will me add a cached hash of the string later without
// changing rest of the code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LoxStr {
    val: Box<str>,
}

impl LoxStr {
    pub fn as_str(&self) -> &str {
        &self.val
    }

    pub fn to_string(&self) -> String {
        self.val.to_string()
    }
}

// impl From<String> for LoxStr {
//     fn from(val: String) -> Self {
//         let val: Box<str> = val.into();
//         Self {val}
//     }
// }

impl Display for LoxStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.val.fmt(f)
    }
}

impl<T> From<T> for LoxStr
where
    Box<str>: From<T>,
{
    fn from(val: T) -> Self {
        let val: Box<str> = val.into();
        Self { val }
    }
}

impl Deref for LoxStr {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for LoxStr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for LoxStr {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Trace for LoxStr {
    fn trace(&self, grey_stack: &mut GreyStack) {
    }
}

pub trait HeapObj: 'static {
    fn is_marked(&self) -> bool;
    fn mark(&mut self);
    fn unmark(&mut self);
}

pub trait Trace {
    fn trace(&self, grey_stack: &mut GreyStack);
}

