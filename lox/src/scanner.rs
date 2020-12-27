use std::{iter::Peekable, str::CharIndices};

pub struct Scanner<'a> {
    source: &'a str,
    curr: Peekable<CharIndices<'a>>,
    start: usize,
    line: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        let curr = source.char_indices().peekable();

        Scanner {
            source,
            curr,
            line: 1,
            start: 0,
        }
    }

    fn consume_if(&mut self, c: char) -> bool {
        let is_match = self.match_char(c);
        if is_match {
            self.curr.next();
        }

        is_match
    }

    // #[inline]
    fn match_char(&mut self, c: char) -> bool {
        match self.curr.peek() {
            None => false,
            Some((_sz, char_)) => *char_ == c,
        }
    }

    // #[inline]
    fn can_match_digit(&mut self) -> bool {
        match self.curr.peek() {
            None => false,
            Some((_sz, char_)) => char_.is_digit(BASE),
        }
    }

    // #[inline]
    fn can_match_alphanumeric(&mut self) -> bool {
        match self.curr.peek() {
            None => false,
            Some((_sz, char_)) => char_.is_alphanumeric(),
        }
    }

    fn make_token(&mut self, kind: TokenType) -> Token<'a> {
        let end_index = match self.curr.peek() {
            None => self.source.len(),
            Some((char_pos, _char)) => *char_pos,
        };

        Token {
            line: self.line,
            kind,
            description: &self.source[self.start..end_index],
        }
    }

    fn error_token(&self, msg: &'static str) -> Token<'a> {
        Token {
            line: self.line,
            kind: TokenType::Error,
            description: msg,
        }
    }

    fn is_at_end(&mut self) -> bool {
        None == self.curr.peek()
    }

    fn skip_whitespace(&mut self) {
        loop {
            if let Some((_sz, a)) = self.curr.peek() {
                match a {
                    ' ' | '\r' | '\t' => {
                        self.curr.next();
                    }
                    '\n' => {
                        self.line += 1;
                        self.curr.next();
                        // // Why break here? What if next line starts with whitespace?
                        // break;
                    }
                    '/' => {
                        if let Some((_, '/')) = self.curr.peek_twice() {
                            self.curr.next();
                            self.curr.next();
                            while !self.is_at_end() && self.match_char('\n') {
                                self.curr.next();
                            }
                        }
                    }
                    _ => {
                        break;
                    }
                };
            } else {
                break;
            }
        }
    }

    fn string(&mut self) -> Token<'a> {
        while !self.is_at_end() && !self.match_char('"') {
            self.curr.next();
        }

        if self.is_at_end() {
            return self.error_token("Unterminated string.");
        }

        self.curr.next();

        return self.make_token(TokenType::String);
    }

    fn get_curr_string(&mut self) -> &str {
        let end = match self.curr.peek() {
            None => self.source.len(),
            Some((char_pos, _char)) => *char_pos,
        };

        &self.source[self.start..end]
    }

    fn digit(&mut self) -> Token<'a> {
        while !self.is_at_end() && self.can_match_digit() {
            self.curr.next();
        }

        let has_dot = self.match_char('.');

        let is_fraction = has_dot && match self.curr.peek_twice() {
            None => false,
            Some((_sz, char_)) => char_.is_digit(BASE),
        };

        if is_fraction {
            self.curr.next();
            while self.can_match_digit() {
                self.curr.next();
            }
        }

        return self.make_token(TokenType::Number);
    }

    fn identifier(&mut self) -> Token<'a> {
        while self.can_match_alphanumeric() {
            self.curr.next();
        }

        let token = identifier_type(self.get_curr_string());
        return self.make_token(token);
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Token<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace();

        self.start = match self.curr.peek() {
            None => self.source.len(),
            Some((char_pos, _char)) => *char_pos,
        };

        if self.is_at_end() {
            // return Some(self.makeToken(T::EOF));
            return None;
        }

        let (_sz, c) = self.curr.next().unwrap();

        if c.is_digit(BASE) {
            return Some(self.digit());
        }

        if c.is_alphabetic() {
            return Some(self.identifier());
        }

        let token = match c {
            '(' => self.make_token(TokenType::LeftParen),
            ')' => self.make_token(TokenType::RightParen),
            '{' => self.make_token(TokenType::LeftBrace),
            '}' => self.make_token(TokenType::RightBrace),
            ',' => self.make_token(TokenType::Comma),
            '.' => self.make_token(TokenType::Dot),
            '-' => self.make_token(TokenType::Minus),
            '+' => self.make_token(TokenType::Plus),
            ';' => self.make_token(TokenType::SemiColon),
            '/' => self.make_token(TokenType::Slash),
            '*' => self.make_token(TokenType::Star),
            '!' => {
                if self.consume_if('=') {
                    self.make_token(TokenType::BangEqual)
                } else {
                    self.make_token(TokenType::Bang)
                }
            }
            '=' => {
                if self.consume_if('=') {
                    self.make_token(TokenType::EqualEqual)
                } else {
                    self.make_token(TokenType::Equal)
                }
            }
            '>' => {
                if self.consume_if('=') {
                    self.make_token(TokenType::GreaterEqual)
                } else {
                    self.make_token(TokenType::Greater)
                }
            }
            '<' => {
                if self.consume_if('=') {
                    self.make_token(TokenType::LessEqual)
                } else {
                    self.make_token(TokenType::Less)
                }
            }
            '"' => self.string(),
            _ => self.error_token("Unexpected character."),
        };

        Some(token)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Token<'a> {
    pub line: usize,
    pub kind: TokenType,
    pub description: &'a str,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TokenType {
    // Single-character tokens.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    SemiColon,
    Slash,
    Star,

    // One or two Character tokens
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals.
    Identifier,
    String,
    Number,

    //
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,

    Error,
    EOF,

    // Placeholder token to represent an uninitialized token
    Placeholder,
}

trait DoubleLookAhead {
    fn peek_twice(&mut self) -> Option<(usize, char)>;
}

impl<'a> DoubleLookAhead for Peekable<CharIndices<'a>> {
    fn peek_twice(&mut self) -> Option<(usize, char)> {
        let mut peek_iter = self.clone();
        peek_iter.next();
        peek_iter.next()
    }
}

const BASE: u32 = 10;

// TODO: Rewrite this function to be more cleaner
fn identifier_type(ident: &str) -> TokenType {
    fn check_match(a: &str, b: &str, kind: TokenType) -> TokenType {
        if a == b {
            return kind;
        } else {
            return TokenType::Identifier;
        }
    }

    let mut chars = ident.chars();
    let c = chars.next().expect("Attempted to create empty identifier");
    let remaining = chars.as_str();

    match c {
        'a' => check_match(remaining, "nd", TokenType::And),
        'c' => check_match(remaining, "omma", TokenType::Comma),
        'e' => check_match(remaining, "lse", TokenType::Else),
        'i' => check_match(remaining, "f", TokenType::If),
        'n' => check_match(remaining, "il", TokenType::Nil),
        'o' => check_match(remaining, "r", TokenType::Or),
        'p' => check_match(remaining, "rint", TokenType::Print),
        'r' => check_match(remaining, "eturn", TokenType::Return),
        's' => check_match(remaining, "uper", TokenType::Super),
        'v' => check_match(remaining, "ar", TokenType::Var),
        'w' => check_match(remaining, "hile", TokenType::While),
        'f' => {
            let nc = chars.next();
            let remaining = chars.as_str();
            if nc.is_some() {
                match nc.unwrap() {
                    'a' => check_match(remaining, "lse", TokenType::False),
                    'o' => check_match(remaining, "r", TokenType::For),
                    'u' => check_match(remaining, "n", TokenType::Fun),
                    _ => TokenType::Identifier,
                }
            } else {
                TokenType::Identifier
            }
        }
        't' => {
            let nc = chars.next();
            let remaining = chars.as_str();
            if nc.is_some() {
                match nc.unwrap() {
                    'h' => check_match(remaining, "is", TokenType::This),
                    'r' => check_match(remaining, "ue", TokenType::True),
                    _ => TokenType::Identifier,
                }
            } else {
                TokenType::Identifier
            }
        }
        _ => TokenType::Identifier,
    }
}

impl Token<'_> {
    pub fn placeholder() -> Self {
        Token {
            kind: TokenType::Placeholder,
            line: 0,
            description: ""
        }
    }
}