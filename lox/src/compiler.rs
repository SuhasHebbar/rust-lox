use crate::{opcodes::{ConstantIndex, Number}, precedence::{PLACEHOLDER_PARSEFN, Precedence, parse_rule}};
use std::{ptr, todo};

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
    had_error: bool,
    panic_mode: bool,
    pub chunk: Chunk,
}

impl<'a> Compiler<'a> {
    pub fn new(src: &'a str) -> Self {
        let scanner = Scanner::new(src);

        Compiler {
            scanner,
            had_error: false,
            panic_mode: false,
            previous: Token::placeholder(),
            current: Token::placeholder(),
            chunk: Chunk::new(),
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

    pub fn grouping(&mut self) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    pub fn unary(&mut self) {
        let op_type = self.previous.kind;

        self.parse_precedence(Precedence::Unary);

        match op_type {
            TokenType::Minus => self.emit_instruction(Instruction::Negate),
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
            _ => panic!("Unsupported binary operator {:?}", op_type),
        }
        // do nothing
    }

    pub fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        let prefix_fn = parse_rule(self.current.kind).prefix;
        self.advance();

        if ptr::eq(prefix_fn, PLACEHOLDER_PARSEFN) {
            self.error_at_previous("Unexpected expression.")
        }

        prefix_fn(self);

        loop {
            let prule = parse_rule(self.current.kind);
            if prule.precedence <= precedence {
                break;
            }

            self.advance();
            (prule.infix)(self);
        }
    }
}
