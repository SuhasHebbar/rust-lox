use std::{fmt::{Display, self, Formatter}, ops::Deref, rc::Rc};

/// Currently this is just the bare beginnings of a scaffold for the lox GC.

#[derive(Debug, Clone)]
pub struct Gc<T> {
    ptr: Rc<T>
}

impl<T> From<T> for Gc<T> {
    fn from(val: T) -> Self {
        Gc {
            ptr: Rc::new(val)
        }
    }
}

impl<T> Deref for Gc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<T> Display for Gc<T> where T: Display {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.ptr.fmt(f)
    }
}