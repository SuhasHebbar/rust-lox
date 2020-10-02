use std::{iter::Peekable, str::Chars};

pub struct Scanner<'a> {
    source: &'a str,
    curr: Peekable<Chars<'a>>,
    line: usize
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        let curr = source.chars().peekable();

        Scanner {
            source,
            curr           ,
            line: 0
        }
    }
}

