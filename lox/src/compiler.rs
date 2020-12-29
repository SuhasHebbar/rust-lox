use crate::{heap::{Heap, LoxStr}, opcodes::{ConstantIndex, Number}, precedence::{parse_rule, ParseFn, Precedence}};
use std::ptr;

use crate::{
    opcodes::Chunk,
    scanner::{Token, TokenType},
};
use crate::{
    opcodes::{Instruction, Value},
    scanner::Scanner,
};

pub struct Compiler<'a> {
    scanner: Scanner<'a>,
    previous: Token<'a>,
    current: Token<'a>,
    pub had_error: bool,
    panic_mode: bool,
    pub chunk: Chunk,
    pub heap: Heap
}

impl<'a> Compiler<'a> {
    pub fn new(src: &'a str) -> Self {
        let scanner = Scanner::new(src);

        Compiler {
            scanner,
            previous: Token::placeholder(),
            current: Token::placeholder(),
            had_error: false,
            panic_mode: false,
            chunk: Chunk::new(),
            heap: Heap::new()
        }
    }

    pub fn compile(&mut self) -> bool {
        self.advance();
        self.expression();

        // This shouldn't be needed as the scanner iterator should return EOF
        // self.consume(EOF, "End of Expression");
        self.end_compile();
        !self.had_error
    }

    fn end_compile(&mut self) {
        self.emit_return();

        #[cfg(feature = "lox_debug")]
        {
            if self.had_error {
                eprintln!("Dumping bytecode to console");
                eprintln!("{}", self.chunk);
            }
        }
    }

    fn consume(&mut self, token_type: TokenType, message: &str) {
        if self.current.kind == token_type {
            self.advance();
        } else {
            self.error_at_current(message);
        }
    }

    fn advance(&mut self) {
        self.previous = self.current;

        loop {
            self.current = self.scanner.scan_token();

            if self.current.kind != TokenType::Error {
                break;
            }

            self.error_at_current(self.current.description);
        }
    }

    fn error_at_current(&mut self, message: &str) {
        Self::error_at(
            &mut self.had_error,
            &mut self.panic_mode,
            &self.current,
            message,
        );
    }

    fn error_at_previous(&mut self, message: &str) {
        Self::error_at(
            &mut self.had_error,
            &mut self.panic_mode,
            &self.previous,
            message,
        );
    }

    fn error_at(had_error: &mut bool, panic_mode: &mut bool, token: &Token, message: &str) {
        if *panic_mode {
            return;
        }

        eprint!("[line {}] Error ", token.line);

        if token.kind == TokenType::Error {
            eprint!("while Scanning");
        } else {
            eprint!("at {}", token.description);
        }

        eprint!(": {}\n", message);
        *had_error = true;
        *panic_mode = true;
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.chunk
    }

    fn emit_instruction(&mut self, instr: Instruction) {
        let line = self.previous.line;
        self.current_chunk().add_instruction(instr, line);
    }

    fn emit_value(&mut self, value: Value) -> u8 {
        self.current_chunk().add_value(value)
    }

    fn emit_return(&mut self) {
        self.emit_instruction(Instruction::Return);
    }

    fn make_constant(&mut self, value: Value) -> ConstantIndex {
        let constant_index = self.emit_value(value);
        if constant_index > u8::MAX {
            self.error_at_previous("Too many constants in one chunk.");
            0
        } else {
            constant_index
        }
    }

    fn emit_constant(&mut self, value: Value) {
        let constant_index = self.make_constant(value);
        self.emit_instruction(Instruction::Constant(constant_index));
    }

    pub fn number(&mut self) {
        let value: Number = self.previous.description.parse().unwrap();
        self.emit_constant(Value::Number(value))
    }

    pub fn literal(&mut self) {
        match self.previous.kind {
            TokenType::False => self.emit_instruction(Instruction::False),
            TokenType::Nil => self.emit_instruction(Instruction::Nil),
            TokenType::True => self.emit_instruction(Instruction::True),
            _ => panic!("Non literal token found in literal() parse"),
        }
    }

    pub fn grouping(&mut self) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    pub fn unary(&mut self) {
        let op_type = self.previous.kind;

        self.parse_precedence(Precedence::Unary);

        match op_type {
            TokenType::Minus => self.emit_instruction(Instruction::Negate),
            TokenType::Bang => self.emit_instruction(Instruction::Not),
            _ => (),
        };
    }

    pub fn binary(&mut self) {
        let op_type = self.previous.kind;

        let prule = parse_rule(op_type);
        self.parse_precedence(prule.precedence.next_greater());

        match op_type {
            TokenType::Plus => self.emit_instruction(Instruction::Add),
            TokenType::Minus => self.emit_instruction(Instruction::Subtract),
            TokenType::Star => self.emit_instruction(Instruction::Multiply),
            TokenType::Slash => self.emit_instruction(Instruction::Divide),
            TokenType::EqualEqual => self.emit_instruction(Instruction::Equal),
            TokenType::BangEqual => {
                self.emit_instruction(Instruction::Equal);
                self.emit_instruction(Instruction::Not);
            }
            TokenType::Greater => self.emit_instruction(Instruction::Greater),
            TokenType::GreaterEqual => {
                self.emit_instruction(Instruction::Less);
                self.emit_instruction(Instruction::Not);
            }
            TokenType::Less => self.emit_instruction(Instruction::Less),
            TokenType::LessEqual => {
                self.emit_instruction(Instruction::Greater);
                self.emit_instruction(Instruction::Not);
            }
            _ => panic!("Unsupported binary operator {:?}", op_type),
        }
        // do nothing
    }

    pub fn string(&mut self) {
        let lexeme_len  = self.previous.description.len();
        let string: LoxStr = self.previous.description[1..lexeme_len - 1].into();
        let string_ref = self.heap.intern_string(string);
        self.emit_constant(Value::String(string_ref));
    }

    pub fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        let prefix_fn = parse_rule(self.current.kind).prefix;

        if let Some(prefix_fn) = prefix_fn {
            self.advance();
            prefix_fn(self);
        } else {
            self.error_at_previous("Unexpected expression.");
            return;
        }

        loop {
            let prule = parse_rule(self.current.kind);
            if prule.precedence <= precedence {
                break;
            }

            self.advance();
            (prule.infix.unwrap())(self);
        }
    }
}
