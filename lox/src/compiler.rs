use crate::{heap::{Heap}, opcodes::{ChunkIterator, ConstantIndex, Number}, precedence::{parse_rule, ParseRule, Precedence}, vm::StackIndex};

use crate::{
    opcodes::Chunk,
    scanner::{Token, TokenType},
};
use crate::{
    opcodes::{Instruction, Value},
    scanner::Scanner,
};

type StringError = &'static str;

pub struct Compiler<'a> {
    scanner: Scanner<'a>,
    tin: TokenCursor<'a>,
    pub chunks: Chunks,
    pub heap: Heap,
    pub state: StackSim<'a>,
    pub errh: ErrorHandler
}

impl<'a> Compiler<'a> {
    pub fn new(src: &'a str) -> Self {
        let scanner = Scanner::new(src);

        Compiler {
            scanner,
            tin: TokenCursor::new(),
            chunks: Chunks::new(),
            heap: Heap::new(),
            state: StackSim::new(),
            errh: ErrorHandler {had_error: false, panic_mode: false}
        }
    }

    pub fn compile(&mut self) -> bool {
        self.advance();

        while !self.match_tt(TokenType::EOF) {
            self.declaration()
        }

        // This shouldn't be needed as the scanner iterator should return EOF
        // self.consume(EOF, "End of Expression");
        self.end_compile();
        !self.errh.had_error
    }

    fn end_compile(&mut self) {
        self.emit_return();

        #[cfg(feature = "lox_debug")]
        {
            if self.errh.had_error {
                eprintln!("Dumping bytecode to console");
                eprintln!("{}", self.chunk);
            }
        }
    }

    fn consume(&mut self, token_type: TokenType, message: &str) {
        if self.tin.cur.kind == token_type {
            self.advance();
        } else {
            self.error_at_current(message);
        }
    }

    fn check(&self, token_type: TokenType) -> bool {
        self.tin.cur.kind == token_type
    }

    fn match_tt(&mut self, token_type: TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) {
        self.tin.pre = self.tin.cur;

        loop {
            self.tin.cur = self.scanner.scan_token();

            if self.tin.cur.kind != TokenType::Error {
                break;
            }

            self.error_at_current(self.tin.cur.description);
        }
    }

    fn add_local(&mut self) {
        if self.state.size() == LOCALS_MAX_CAPACITY {
            self.error_at_previous("Too many local variables in function.");
            return;
        }

        self.state.add_local(self.tin.pre);
    }

    fn error_at_current(&mut self, message: &str) {
        self.errh.error_at_current(&self.tin, message);
    }

    fn error_at_previous(&mut self, message: &str) {
        self.errh.error_at_previous(&self.tin, message);
    }

    fn emit_instruction(&mut self, instr: Instruction) {
        self.chunks.emit_instruction(instr, &self.tin.pre)
    }

    fn emit_return(&mut self) {
        self.emit_instruction(Instruction::Return);
    }

    fn make_constant(chunks: &mut Chunks, value: Value, errh: &mut ErrorHandler, cursor: &TokenCursor) -> ConstantIndex {
        let constant_index = chunks.emit_value(value);
        if constant_index > u8::MAX {
            errh.error_at_previous(cursor, "Too many constants in one chunk.");
            0
        } else {
            constant_index
        }
    }

    fn emit_constant(&mut self, value: Value) {
        let constant_index = Self::make_constant(&mut self.chunks, value, &mut self.errh, &self.tin);
        self.emit_instruction(Instruction::Constant(constant_index));
    }

    fn synchronize(&mut self) {
        self.errh.panic_mode = false;

        while self.tin.cur.kind != TokenType::EOF {
            if self.tin.pre.kind == TokenType::SemiColon {
                return;
            }

            match self.tin.cur.kind {
                TokenType::Class
                | TokenType::Fun
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => {
                    return;
                }
                _ => self.advance(),
            }
        }
    }

    pub fn number(&mut self) {
        let value: Number = self.tin.pre.description.parse().unwrap();
        self.emit_constant(Value::Number(value))
    }

    pub fn literal(&mut self) {
        match self.tin.pre.kind {
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
        let op_type = self.tin.pre.kind;

        self.parse_precedence(Precedence::Unary);

        match op_type {
            TokenType::Minus => self.emit_instruction(Instruction::Negate),
            TokenType::Bang => self.emit_instruction(Instruction::Not),
            _ => (),
        };
    }

    pub fn binary(&mut self) {
        let op_type = self.tin.pre.kind;

        let prule = parse_rule(op_type);
        self.parse_precedence(prule.curr_prec.next_greater());

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
        let lexeme_len = self.tin.pre.description.len();
        let string = &self.tin.pre.description[1..lexeme_len - 1];
        let string_ref = self.heap.intern_string(string);
        self.emit_constant(Value::String(string_ref));
    }

    pub fn variable(&mut self, assign: bool) {
        let arg = self.resolve_local();

        let set_op;
        let get_op;

        if let Some(arg) = arg {
            get_op = Instruction::GetLocal(arg);
            set_op = Instruction::SetLocal(arg);
        } else {
            let var_index = self.make_identifier();
            set_op = Instruction::SetGlobal(var_index);
            get_op = Instruction::GetGlobal(var_index);
        }

        if self.match_tt(TokenType::Equal) && assign {
            self.expression();

            self.emit_instruction(set_op);
        } else {
            self.emit_instruction(get_op);
        }
    }

    fn resolve_local(&mut self) -> Option<StackIndex> {
        for (i, local) in self.state.locals.iter().enumerate().rev() {
            if local.name.description == self.tin.pre.description {
                if local.depth == -1 {

                    self.error_at_previous("Can't read local variable in its own initializer.");
                }

                return Some(i as u8);
            }
        }

        None
    }

    pub fn declaration(&mut self) {
        if self.match_tt(TokenType::Var) {
            self.var_declaration()
        } else {
            self.statement();
        }

        if self.errh.panic_mode {
            self.synchronize();
        }
    }

    pub fn var_declaration(&mut self) {
        let var_name_index = self.parse_variable("Expect variable name.");

        if self.match_tt(TokenType::Equal) {
            self.expression();
        } else {
            self.emit_instruction(Instruction::Nil);
        }

        self.consume(
            TokenType::SemiColon,
            "Expect ';' after variable declaration.",
        );

        self.define_variable(var_name_index);
    }

    fn define_variable(&mut self, global: ConstantIndex) {
        if self.state.scope_depth > 0 {
            self.state.mark_initialized();
        } else {
            self.emit_instruction(Instruction::DefineGlobal(global));
        }
    }

    fn parse_variable(&mut self, msg: &str) -> ConstantIndex {
        self.consume(TokenType::Identifier, msg);

        self.declare_variable();
        if self.state.scope_depth > 0 {
            return 0;
        }

        self.make_identifier()
    }

    fn declare_variable(&mut self) {
        if self.state.scope_depth == 0 {
            return;
        }

        for (i, local) in self.state.locals.iter().enumerate().rev() {
            if local.depth != -1 && local.depth < self.state.scope_depth {
                break;
            }

            if local.name.description == self.tin.pre.description {
                self.errh.error_at(&self.tin.pre, "Already variable with this name in this scope.");
            }
        }

        self.add_local();
    }

    fn make_identifier(&mut self) -> ConstantIndex {
        let lox_str = self.heap.intern_string(self.tin.pre.description);
        Self::make_constant(&mut self.chunks, Value::String(lox_str), &mut self.errh, &self.tin)
    }

    pub fn statement(&mut self) {
        if self.match_tt(TokenType::Print) {
            self.print_statement();
        } else if self.match_tt(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
            self.consume(TokenType::SemiColon, "Expect ';' after value.");
            self.emit_instruction(Instruction::Pop);
        }
    }

    fn begin_scope(&mut self) {
        self.state.begin_scope();
    }

    fn end_scope(&mut self) {
        self.state.end_scope();

        for i in (0..self.state.size()).rev() {
            let local = &self.state.locals[i];
            if self.state.scope_depth < local.depth {
                self.state.locals.pop();
                self.emit_instruction(Instruction::Pop);
            } else {
                break;
            }
        }
        // for local in self.state.locals.iter().rev() {
        //     if self.state.scope_depth < local.depth {
        //         self.state.locals.pop();
        //         self.emit_instruction(Instruction::Pop);
        //     }
        // }
    }

    pub fn block(&mut self) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::EOF) {
            self.declaration();
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.");
    }

    pub fn expression_statement(&mut self) {
        self.expression();
    }

    pub fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::SemiColon, "Expect ';' after value.");
        self.emit_instruction(Instruction::Print);
    }

    pub fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn parse_precedence(&mut self, prec_bound: Precedence) {
        let ParseRule {
            prefix: prefix_fn,
            curr_prec,
            ..
        } = parse_rule(self.tin.cur.kind);
        let can_assign = *curr_prec <= Precedence::Assignment;

        if let Some(prefix_fn) = prefix_fn {
            self.advance();
            prefix_fn(self, can_assign);
        } else {
            self.error_at_previous("Unexpected expression.");
            return;
        }

        loop {
            let prule = parse_rule(self.tin.cur.kind);
            if prec_bound <= prule.curr_prec {
                self.advance();
                (prule.infix.unwrap())(self, can_assign);
            } else {
                break;
            }
        }
    }
}

pub struct StackSim<'a> {
    pub locals: Vec<Local<'a>>,
    pub scope_depth: isize,
}

const LOCALS_MAX_CAPACITY: usize = u8::MAX as usize;

impl<'a> StackSim<'a> {
    fn new() -> Self {
        Self {
            locals: Vec::with_capacity(LOCALS_MAX_CAPACITY),
            scope_depth: 0,
        }
    }

    fn add_local(&mut self, token: Token<'a>) {
        self.locals.push(Local {
            name: token,
            depth: -1,
        });
    }

    fn mark_initialized(&mut self) {
        let len = self.size() - 1;
        self.locals[len].depth = self.scope_depth;
    }

    fn size(&self) -> usize {
        self.locals.len()
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;
    }
}

pub struct Local<'a> {
    name: Token<'a>,
    depth: isize,
}

pub struct ErrorHandler {
    pub panic_mode: bool,
    pub had_error: bool,
}

impl ErrorHandler {
    fn error_at_previous(&mut self, cursor: &TokenCursor, message: &str) {
        self.error_at(&cursor.pre, message);
    }

    fn error_at_current(&mut self, cursor: &TokenCursor, message: &str) {
        self.error_at(&cursor.cur, message);
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
}

pub struct Chunks {
    chunk: Chunk
}

impl Chunks {
    fn new() -> Self {
        Self {chunk: Chunk::new()}
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.chunk
    }

    fn emit_instruction(&mut self, instr: Instruction, token: &Token) {
        let line = token.line;
        self.current_chunk().add_instruction(instr, line);
    }

    fn emit_value(&mut self, value: Value) -> ConstantIndex {
        self.current_chunk().add_value(value)
    }
}

struct TokenCursor<'a> {
    cur: Token<'a>,
    pre: Token<'a>,
}

impl TokenCursor<'_> {
    fn new() -> Self {
        Self {
            cur: Token::placeholder(),
            pre: Token::placeholder(),
        }
    }
}