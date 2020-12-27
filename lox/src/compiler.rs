use std::todo;
use crate::{opcodes::Number, precedence::Precedence};

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
    chunk: Chunk,
}

impl Compiler<'_> {
    fn new(src: &str) -> Self {
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

    fn compile(&mut self) -> bool {
        self.advance();
        // self.expression();

        // This shouldn't be needed as the scanner iterator should return EOF
        // self.consume(EOF, "End of Expression");
        self.end_compile();
        !self.had_error
    }

    fn end_compile(&mut self) {
        self.emit_return();
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
            // I should handle the None at the end of iteration here?
            self.current = self.scanner.next().unwrap();

            if self.current.kind != TokenType::Error {
                break;
            }

            self.error_at_current(self.current.description);
        }
    }

    fn error_at_current(&mut self, message: &str) {
        self.error_at(self.current, message);
    }

    fn error_at_previous(&mut self, message: &str) {
        self.error_at(self.previous, message);
    }

    fn error_at(&mut self, token: &Token, message: &str) {
        if self.panic_mode {
            return;
        }

        eprint!("[line {}] Error ", token.line);

        if token.kind == TokenType::Error {
            eprint!("while Scanning");
        } else {
            eprint!("at {}", token.description);
        }

        eprint!(": {}\n", message);
        self.had_error = true;
        self.panic_mode = true;
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        self.chunk
    }

    fn emit_instruction(&mut self, instr: Instruction) {
        self.current_chunk()
            .add_instruction(instr, self.previous.line);
    }

    fn emit_value(&mut self, value: Value) -> u8 {
        self.current_chunk().add_value(value)
    }

    fn emit_return(&mut self) {
        self.emit_instruction(Instruction::Return);
    }

    fn make_constant(&mut self, value: Value) {
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
        let value: Number = self.previous.description.parse();
        self.emit_constant(value)
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

    pub fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn parse_precedence(&mut self, Precedence: Precedence) {
        todo!()
    }
}
