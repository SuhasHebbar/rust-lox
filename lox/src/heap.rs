/// Currently this is just the bare beginnings of a scaffold for the lox GC.
use std::{borrow::{Borrow, BorrowMut}, cell::RefCell, collections::{HashMap, HashSet}, fmt::{self, Display, Formatter}, hash::Hasher, ops::Deref, ptr::NonNull, rc::Rc};
use std::{hash::Hash, mem};

pub struct Heap {
    interned_strs: RefCell<HashMap<&'static LoxStr, Box<HeapObj<LoxStr>>>>,
}

// impl Drop for Heap {
//     fn drop(&mut self) {
//         let heap = self.interned_strs.borrow_mut();
//         dbg!("{}\n", &*heap);
//     }
// }

impl Heap {
    pub fn new() -> Self {
        Self {
            interned_strs: RefCell::new(HashMap::new()),
        }
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
            obj_ptr = heapobj.as_mut() as *mut HeapObj<LoxStr>;
        } else {
            drop(heapobj);
            drop(interned_strs);
            let mut value = Box::new(HeapObj::new(string));
            obj_ptr = value.as_mut() as *mut HeapObj<LoxStr>;
            let new_key = unsafe { mem::transmute(&value.data) };
            self.interned_strs.borrow_mut().insert(new_key, value);
        }
        Gc::from(obj_ptr)
    }
}

#[derive(Clone, Debug)]
pub struct HeapObj<T: ?Sized> {
    data: T,
}

impl<T> HeapObj<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T> Hash for HeapObj<T> where T: Hash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Gc<T> {
    ptr: NonNull<HeapObj<T>>,
}

impl<T> Copy for Gc<T> {}
impl<T> Clone for Gc<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Gc<T> {
    fn as_obj(&self) -> &HeapObj<T> {
        unsafe { &self.ptr.as_ref() }
    }

    fn as_obj_mut(&mut self) -> &mut HeapObj<T> {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> From<*mut HeapObj<T>> for Gc<T> {
    fn from(val: *mut HeapObj<T>) -> Self {
        let ptr = unsafe { NonNull::new_unchecked(val) };
        Self { ptr }
    }
}

impl<T> Deref for Gc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.as_obj().data
    }
}

impl<T> Borrow<T> for Gc<T> {
    fn borrow(&self) -> &T {
        &self.as_obj().data
    }
}

impl<T> BorrowMut<T> for Gc<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.as_obj_mut().data
    }
}

impl<T> AsRef<T> for Gc<T> {
    fn as_ref(&self) -> &T {
        &self.as_obj().data
    }
}

impl<T> AsMut<T> for Gc<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.as_obj_mut().data
    }
}


impl<T> Display for Gc<T>
where
    T: Display,
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