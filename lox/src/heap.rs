/// Currently this is just the bare beginnings of a scaffold for the lox GC.
use std::{borrow::{Borrow, BorrowMut}, cell::{Cell, RefCell}, cmp::max, collections::{HashMap, HashSet}, fmt::{self, Display, Formatter}, hash::Hasher, ops::{Deref, DerefMut}, ptr::NonNull, rc::Rc, todo};
use std::{hash::Hash, mem};

use mem::size_of_val;

use crate::{object, vm::Vm};

pub type GreyStack = Vec<&'static dyn Trace>;

const GC_HEAP_GROWTH_FACTOR: usize = 2;
const INITIAL_NEXT_GC: usize = 1024 * 1024;

pub struct Heap {
    interned_strs: RefCell<HashMap<&'static LoxStr, Box<Obj<LoxStr>>>>,
    objects: RefCell<Vec<Box<dyn HeapObj>>>,
    grey_stack: RefCell<GreyStack>,
    bytes_allocated: Cell<usize>,
    next_gc: Cell<usize>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            interned_strs: RefCell::new(HashMap::new()),
            objects: RefCell::new(Vec::new()),
            grey_stack: RefCell::new(Vec::new()),
            bytes_allocated: Cell::new(0),
            next_gc: Cell::new(INITIAL_NEXT_GC),
        }
    }

    fn collect_if_needed(&self, vm: &Vm) {
        #[cfg(feature = "debug_stress_gc")]
        self.collect_garbage(vm);

        let total_bytes_allocated = self.bytes_allocated.get();
        let next_gc = self.next_gc.get();

        if total_bytes_allocated > next_gc {
            self.collect_garbage(vm);
        }
    }

    fn collect_garbage(&self, vm: &Vm) {
        let bytes_allocated_prev: usize;
        #[cfg(feature = "debug_log_gc")]
        {
            println!("-- gc begin");
            bytes_allocated_prev = self.bytes_allocated.get();
        }

        self.mark_heap(vm);
        self.sweep_heap();

        let next_gc = max(INITIAL_NEXT_GC, self.bytes_allocated.get() * GC_HEAP_GROWTH_FACTOR);
        self.next_gc.replace(next_gc);

        #[cfg(feature = "debug_log_gc")]
        {
            println!("-- gc end");
            let curr_allocated = self.bytes_allocated.get();
            println!(
                "   Collected {} bytes (from {} to {}). Next at {}",
                bytes_allocated_prev - curr_allocated ,
                bytes_allocated_prev,
                curr_allocated,
                next_gc
            );
        }
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

        vm.class_init_method.mark_if_needed(grey_stack);
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

        let mut objects_size = 0usize;
        for object in objects.iter_mut() {
            object.unmark();
            objects_size += object.bytes_allocated();
        }

        let mut interned_strs = self.interned_strs.borrow_mut();

        interned_strs.retain(|k, v| v.is_marked());

        let mut strs_size = 0;
        for (k, v) in interned_strs.iter_mut() {
            v.unmark();
            strs_size += v.bytes_allocated();

        }

        self.bytes_allocated.replace(strs_size + objects_size);
    }

    pub fn manage_gc<T: Trace>(&self, value: T, vm: &Vm) -> Gc<T> {
        self.collect_if_needed(vm);
        self.manage(value)
    }

    pub fn intern_string_gc(&self, str_ref: impl AsRef<str>, vm: &Vm) -> Gc<LoxStr> {
        self.collect_if_needed(vm);
        self.intern_string(str_ref)
    }

    pub fn manage<T: Trace>(&self, value: T) -> Gc<T> {
        let mut boxed = Box::new(Obj::new(value));
        let ptr = boxed.as_mut() as *mut _;

        let bytes_allocated = boxed.bytes_allocated();

        let total_bytes_allocated = bytes_allocated + self.bytes_allocated.get();
        self.bytes_allocated.replace(total_bytes_allocated);

        #[cfg(feature = "debug_log_allocation")]
        println!(
            "Allocate {:?}, size = {}, type = {}",
            ptr,
            bytes_allocated,
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
            let mut boxed = Box::new(Obj::new(string));
            obj_ptr = boxed.as_mut() as *mut Obj<LoxStr>;

            // Update bytes allocated
            let bytes_allocated = boxed.bytes_allocated();
            let total_bytes_allocated = bytes_allocated + self.bytes_allocated.get();
            self.bytes_allocated.replace(total_bytes_allocated);
            #[cfg(feature = "debug_log_allocation")]
            println!(
                "Allocate {:?}, size = {}, type = {}",
                obj_ptr, bytes_allocated, "LoxStr"
            );

            let new_key = unsafe { mem::transmute(&boxed.data) };
            self.interned_strs.borrow_mut().insert(new_key, boxed);
        }
        Gc::from(obj_ptr)
    }

    // Some allocated objects may grow in size in response to certain actions. For example setting a field
    // will grow the hashmap used. Any action performed here should keep in mind that call this function may trigger the GC.
    pub fn update_allocation<T: Trace>(&self, obj: Gc<T>, mut action: impl FnMut(), vm: &Vm) {
        let curr_size = obj.bytes_allocated();
        action();
        let new_size = obj.bytes_allocated();

        let new_bytes_allocated = self.bytes_allocated.get() + new_size - curr_size;
        self.bytes_allocated.replace(new_bytes_allocated);

        self.collect_if_needed(vm);
    }
}

#[derive(Clone, Debug)]
pub struct Obj<T: 'static + Trace> {
    marked: bool,
    data: T,
}

#[cfg(feature = "debug_log_allocation")]
impl<T> Drop for Obj<T> where T: Trace {
    fn drop(&mut self) {
        let ptr = self as *const Self;
        let size = self.bytes_allocated();
        println!("Free {:?}, size = {}, type = {}", ptr, size, std::any::type_name::<T>());
    }
}

impl<T: Trace> HeapObj for Obj<T> {
    fn is_marked(&self) -> bool {
        self.marked
    }

    fn mark(&mut self) {
        self.marked = true;
    }

    fn unmark(&mut self) {
        self.marked = false;
    }

    fn bytes_allocated(&self) -> usize {
        self.data.bytes_allocated()
    }
}

impl<T: Trace> Obj<T> {
    pub fn new(data: T) -> Self {
        Self {
            marked: false,
            data,
        }
    }
}

impl<T> Hash for Obj<T>
where
    T: Hash + Trace,
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
    fn trace(&self, _grey_stack: &mut GreyStack) {}

    fn bytes_allocated(&self) -> usize {
        let str_size = mem::size_of_val(self.val.as_ref());
        let box_size = mem::size_of::<LoxStr>();

        str_size + box_size
    }
}

pub trait HeapObj: 'static {
    fn is_marked(&self) -> bool;
    fn mark(&mut self);
    fn unmark(&mut self);

    fn bytes_allocated(&self) -> usize;
}

pub trait Trace {
    fn trace(&self, grey_stack: &mut GreyStack);

    fn bytes_allocated(&self) -> usize;
}
