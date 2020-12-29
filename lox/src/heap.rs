/// Currently this is just the bare beginnings of a scaffold for the lox GC.
use std::{
    borrow::Borrow,
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt::{self, Display, Formatter},
    hash::Hasher,
    ops::Deref,
    ptr::NonNull,
    rc::Rc,
};
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

    pub fn intern_string(&self, string: LoxStr) -> Gc<LoxStr> {
        let has_key = self.interned_strs.borrow().contains_key(&string);
        let mut key;
        if !has_key {
            let value = Box::new(HeapObj::new(string));
            let new_key = unsafe { mem::transmute(&value.data) };
            self.interned_strs.borrow_mut().insert(new_key, value);
            key = new_key;
        } else {
            key = &string;
        }
        Gc::from(self.interned_strs.borrow_mut().get_mut(key).unwrap().as_mut() as *mut HeapObj<LoxStr>)
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

// impl<T> Hash for HeapObj<T> where T: Hash {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         self.data.hash(state);
//     }
// }

#[derive(Debug, Clone)]
pub struct Gc<T> {
    ptr: NonNull<HeapObj<T>>,
}

impl<T> Gc<T> {
    fn as_obj(&self) -> &HeapObj<T> {
        unsafe { &self.ptr.as_ref() }
    }

    fn as_obj_mut(&mut self) -> &mut HeapObj<T> {
        unsafe { self.ptr.as_mut() }
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

impl<T> From<*mut HeapObj<T>> for Gc<T> {
    fn from(val: *mut HeapObj<T>) -> Self {
        let ptr = unsafe { NonNull::new_unchecked(val) };
        Self { ptr }
    }
}

impl<T> Deref for Gc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.as_ref()
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
        self.val.as_ref()
    }
}

impl Borrow<str> for LoxStr {
    fn borrow(&self) -> &str {
        self.val.as_ref()
    }
}

impl AsRef<str> for LoxStr {
    fn as_ref(&self) -> &str {
        self.val.as_ref()
    }
}
