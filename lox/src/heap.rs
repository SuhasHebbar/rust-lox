use std::{borrow::Borrow, fmt::{self, Display, Formatter}, ops::Deref, rc::Rc};

/// Currently this is just the bare beginnings of a scaffold for the lox GC.

#[derive(Debug, Clone)]
pub struct Gc<T> {
    ptr: Rc<T>,
}

impl<T> From<T> for Gc<T> {
    fn from(val: T) -> Self {
        Gc { ptr: Rc::new(val) }
    }
}

impl<T> Deref for Gc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<T> Display for Gc<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.ptr.fmt(f)
    }
}

// Adding wrapper since this will me add a cached hash of the string later without 
// changing rest of the code.
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl<T> From<T> for LoxStr where Box<str>: From<T> {
    fn from(val: T) -> Self {
        let val: Box<str> = val.into();
        Self {val}
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
