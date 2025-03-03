use std::{convert::{TryInto, identity}, todo};

use crate::{
    heap::{Gc, Heap, LoxStr},
    object::{FunctionType, LoxFun, UpvalueSim},
    opcodes::{ArgCount, ByteCodeOffset, ChunkIterator, ConstantIndex, Number},
    precedence::{parse_rule, ParseRule, Precedence},
    vm::StackIndex,
};

macro_rules! cctx {
    ($self: ident) => {
        $self.ctx_stk[$self.curr_ctx]
    };
}

macro_rules! cchunk {
    ($self: ident) => {
        $self.ctx_stk[$self.curr_ctx].function.chunk
    };
}

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
    ctx_stk: Vec<CompilerContext<'a>>,
    curr_ctx: usize,
    class_ctxs: Vec<ClassContext<'a>>,
    pub heap: Heap,
}

impl<'a> Compiler<'a> {
    pub fn new(src: &'a str) -> Self {
        let scanner = Scanner::new(src);
        let heap = Heap::new();
        let empty_string = heap.intern_string("");
        let ctx = CompilerContext::new(FunctionType::Script, empty_string);

        Compiler {
            scanner,
            tin: TokenCursor::new(),
            ctx_stk: vec![ctx],
            curr_ctx: 0,
            class_ctxs: Vec::new(),
            heap,
        }
    }

    pub fn compile(&mut self) -> Option<Gc<LoxFun>> {
        self.advance();

        while !self.match_tt(TokenType::EOF) {
            self.declaration()
        }

        // This shouldn't be needed as the scanner iterator should return EOF
        // self.consume(EOF, "End of Expression");
        self.end_compile()
    }

    fn end_compile(&mut self) -> Option<Gc<LoxFun>> {
        self.emit_return();

        #[cfg(feature = "lox_debug")]
        {
            let ctx = &cctx!(self);
            eprintln!("Dumping bytecode to console");
            eprintln!(
                "{:?}: {} \n{}",
                ctx.function_type,
                ctx.function.name,
                &cchunk!(self)
            );
            // if ctx.errh.had_error {
            //     eprintln!("Dumping bytecode to console");
            //     eprintln!("{:?}: {} \n{}", ctx.function_type, ctx.function.name, &cchunk!(self));
            // }
        }

        if cctx!(self).errh.had_error {
            None
        } else {
            self.curr_ctx = if self.curr_ctx == 0 {
                0
            } else {
                self.curr_ctx - 1
            };
            let CompilerContext {
                mut function,
                upvalues,
                ..
            } = self.ctx_stk.pop().unwrap();
            function.upvalues = upvalues.into();
            let func_ptr = self.heap.manage(function);
            Some(func_ptr)
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
        let ctx = &mut cctx!(self);
        if ctx.stack_sim.size() == LOCALS_MAX_CAPACITY {
            self.error_at_previous("Too many local variables in function.");
            return;
        }

        ctx.stack_sim.add_local(self.tin.pre);
    }

    fn add_specified_local(&mut self, name: Token<'a>) {
        let ctx = &mut cctx!(self);
        if ctx.stack_sim.size() == LOCALS_MAX_CAPACITY {
            self.error_at_previous("Too many local variables in function.");
            return;
        }

        ctx.stack_sim.add_local(name);
    }

    fn error_at_current(&mut self, message: &str) {
        cctx!(self).errh.error_at_current(&self.tin, message);
    }

    fn error_at_previous(&mut self, message: &str) {
        cctx!(self).errh.error_at_previous(&self.tin, message);
    }

    fn emit_instruction(&mut self, instr: Instruction) {
        cctx!(self).emit_instruction(instr, &self.tin);
    }

    fn emit_pop(&mut self) {
        cctx!(self).emit_pop(&self.tin);
    }

    fn emit_return(&mut self) {
        if cctx!(self).function_type == FunctionType::Initializer {
            self.emit_instruction(Instruction::GetLocal(0));
        } else {
            self.emit_instruction(Instruction::Nil);
        }

        self.emit_instruction(Instruction::Return);
    }

    fn make_constant(
        ctx: &mut CompilerContext,
        value: Value,
        cursor: &TokenCursor,
    ) -> ConstantIndex {
        let constant_index = ctx.function.chunk.add_value(value);
        if constant_index > u8::MAX {
            ctx.errh
                .error_at_previous(cursor, "Too many constants in one chunk.");
            0
        } else {
            constant_index
        }
    }

    fn emit_constant(&mut self, value: Value) {
        let constant_index = Self::make_constant(&mut cctx!(self), value, &self.tin);
        self.emit_instruction(Instruction::LoadConstant(constant_index));
    }

    fn synchronize(&mut self) {
        cctx!(self).errh.panic_mode = false;

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

    pub fn call(&mut self) {
        let arg_Count = self.argument_count();
        self.emit_instruction(Instruction::Call(arg_Count));
    }

    fn argument_count(&mut self) -> ArgCount {
        let mut arg_count: ArgCount = 0;
        if !self.check(TokenType::RightParen) {
            loop {
                self.expression();

                if arg_count == ArgCount::MAX {
                    cctx!(self)
                        .errh
                        .error_at_previous(&self.tin, "Can't have more than 255 arguments.");
                }
                arg_count += 1;

                if !self.match_tt(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after arguments.");

        arg_count
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

    pub fn and(&mut self) {
        let patch_loc = self.emit_jump(Instruction::jump_if_false_placeholder());
        self.emit_pop();
        self.parse_precedence(Precedence::And);

        self.patch_fwd_jump(patch_loc);
    }

    pub fn or(&mut self) {
        let jmpif_patch_loc = self.emit_jump(Instruction::jump_if_false_placeholder());
        let jmp_patch_loc = self.emit_jump(Instruction::jump_placeholder());

        self.patch_fwd_jump(jmpif_patch_loc);
        self.emit_pop();

        self.parse_precedence(Precedence::Or);
        self.patch_fwd_jump(jmp_patch_loc);
    }

    pub fn dot(&mut self, assign: bool) {
        self.consume(TokenType::Identifier, "Expect property name after '.'.");
        let rhs_in = self.make_identifier();

        if assign && self.match_tt(TokenType::Equal) {
            self.expression();
            self.emit_instruction(Instruction::SetProperty(rhs_in));
        } else if self.match_tt(TokenType::LeftParen) {
            let arg_count = self.argument_count();
            self.emit_instruction(Instruction::Invoke(rhs_in, arg_count));
        } else {
            self.emit_instruction(Instruction::GetProperty(rhs_in));
        }
    }

    pub fn super_(&mut self) {
        if self.class_ctxs.is_empty() {
            self.error_at_previous("Can't use 'super' outside of a class.");
        } else if !self.class_ctxs.last().unwrap().has_superclass {
            self.error_at_previous("Can't use 'super' in a class with no superclass.");
        }

        self.consume(TokenType::Dot, "Expect '.' after 'super'.");
        self.consume(TokenType::Identifier, "Expect superclass method name.");

        let method_name_in = self.make_identifier();

        self.named_variable("this", false);
        
        if self.match_tt(TokenType::LeftParen) {
            let arg_count = self.argument_count();

            self.named_variable("super", false);
            self.emit_instruction(Instruction::SuperInvoke(method_name_in, arg_count));
        } else {
            self.named_variable("super", false);
            self.emit_instruction(Instruction::GetSuper(method_name_in));
        }

    }

    pub fn this(&mut self) {
        if self.class_ctxs.is_empty() {
            self.error_at_previous("Can't use 'this' outside of a class.");
        }
        self.variable(false);
    }

    fn named_variable(&mut self, name: &str, assign: bool) {
        let arg = self.resolve_local(name);

        let set_op;
        let get_op;

        if let Some(arg) = arg {
            get_op = Instruction::GetLocal(arg);
            set_op = Instruction::SetLocal(arg);
        } else {
            let upvalue = self.resolve_upvalue(self.ctx_stk.len() - 1, name);

            if let Some(upvalue) = upvalue {
                get_op = Instruction::GetUpvalue(upvalue);
                set_op = Instruction::SetUpvalue(upvalue);
            } else {
                let var_index = self.make_identifier_from_name(name);
                set_op = Instruction::SetGlobal(var_index);
                get_op = Instruction::GetGlobal(var_index);
            }
        }

        if self.match_tt(TokenType::Equal) && assign {
            self.expression();

            self.emit_instruction(set_op);
        } else {
            self.emit_instruction(get_op);
        }
 
    }

    pub fn variable(&mut self, assign: bool) {
        self.named_variable(self.tin.pre.description, assign);
   }

    fn resolve_upvalue(&mut self, ctx_in: usize, name: &str) -> Option<StackIndex> {
        if ctx_in == 0 {
            return None;
        }

        let enclosing_ctx = &mut self.ctx_stk[ctx_in - 1];

        let local = enclosing_ctx.resolve_local(&self.tin, name);

        if let Some(local_index) = local {
            enclosing_ctx.stack_sim.locals[local_index as usize].captured = true;
            let local = UpvalueSim::Local(local_index);
            return self.add_upvalue(ctx_in, local);
        }

        let upvalue = self.resolve_upvalue(ctx_in - 1, name);

        if let Some(upvalue) = upvalue {
            let upvalue = UpvalueSim::Upvalue(upvalue);
            return self.add_upvalue(ctx_in, upvalue);
        }

        None
    }

    fn add_upvalue(&mut self, ctx_in: usize, upvalue: UpvalueSim) -> Option<StackIndex> {
        let ctx = &mut self.ctx_stk[ctx_in];
        if let Some(pos) = ctx.upvalues.iter().position(|element| *element == upvalue) {
            Some(pos as u8)
        } else if ctx.upvalues.len() == StackIndex::MAX as usize {
            self.error_at_previous("Too many closure variables in function.");
            Some(0)
        } else {
            ctx.upvalues.push(upvalue);
            Some((ctx.upvalues.len() - 1) as u8)
        }
    }

    fn resolve_local(&mut self, name: &str) -> Option<StackIndex> {
        cctx!(self).resolve_local(&self.tin, name)
    }

    fn fun_declaration(&mut self) {
        let global = self.parse_variable("Expect function name.");
        cctx!(self).stack_sim.mark_initialized();
        self.function(FunctionType::Function);
        self.define_variable(global);
    }

    fn new_context(
        heap: &Heap,
        tin: &TokenCursor,
        function_type: FunctionType,
    ) -> CompilerContext<'a> {
        let name = heap.intern_string(tin.pre.description);
        CompilerContext::new(function_type, name)
    }

    fn function(&mut self, function_type: FunctionType) {
        self.ctx_stk
            .push(Self::new_context(&self.heap, &self.tin, function_type));
        self.curr_ctx += 1;

        cctx!(self).stack_sim.begin_scope();

        self.consume(TokenType::LeftParen, "Expect '(' after function name.");

        if !self.check(TokenType::RightParen) {
            loop {
                let ctx = &mut cctx!(self);
                ctx.function.arity += 1;

                if ctx.function.arity > 255 {
                    ctx.errh
                        .error_at_current(&self.tin, "Can't have more than 255 parameters.");
                }

                let param_constant = self.parse_variable("Expect parameter name.");
                self.define_variable(param_constant);

                if !self.match_tt(TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expect ')' after parameters.");

        self.consume(TokenType::LeftBrace, "Expect '{' before function body.");

        self.block();

        let func_ptr = self.end_compile();

        let func_index = if let Some(func_ptr) = func_ptr {
            Self::make_constant(&mut cctx!(self), Value::Function(func_ptr), &self.tin)
        } else {
            Self::make_constant(&mut cctx!(self), Value::Function(Gc::dangling()), &self.tin)
        };

        self.emit_instruction(Instruction::Closure(func_index));
    }

    pub fn declaration(&mut self) {
        if self.match_tt(TokenType::Var) {
            self.var_declaration()
        } else if self.match_tt(TokenType::Fun) {
            self.fun_declaration();
        } else if self.match_tt(TokenType::Class) {
            self.class_declaration();
        } else {
            self.statement();
        }

        if cctx!(self).errh.panic_mode {
            self.synchronize();
        }
    }

    fn synthetic_token(&mut self, name: &'static str) -> Token<'a> {
        Token {
            kind: TokenType::Identifier,
            line: 0,
            description: name
        }
    }

    fn class_declaration(&mut self) {
        self.consume(TokenType::Identifier, "Expect class name.");
        let class_name_in = self.make_identifier();
        self.declare_variable();

        self.emit_instruction(Instruction::Class(class_name_in));
        self.define_variable(class_name_in);

        let class_name_token = self.tin.pre;
        self.class_ctxs.push(ClassContext::new(&class_name_token));

        if self.match_tt(TokenType::Less) {
            self.consume(TokenType::Identifier , "Expect superclass name.");

            self.begin_scope();
            let synthetic_token = self.synthetic_token("super");
            self.add_specified_local(synthetic_token);
            self.define_variable(0);

            // Put parent class object onto stack.
            self.variable(false);

            if class_name_token.description == self.tin.pre.description {
                self.error_at_previous("A class can't inherit from itself.");
            }

            self.named_variable(class_name_token.description, false);
            self.emit_instruction(Instruction::Inherit);

            self.class_ctxs.last_mut().unwrap().has_superclass = true;
        }

        
        self.named_variable(class_name_token.description, false);

        self.consume(TokenType::LeftBrace, "Expect '{' before class body.");

        while !self.check(TokenType::RightBrace) && !self.check(TokenType::EOF) {
            self.method();
        }

        self.consume(TokenType::RightBrace, "Expect '}' after class body.");

        // Pop class object from self.variable()
        self.emit_pop();

        if self.class_ctxs.last().unwrap().has_superclass {
            self.end_scope();
        }
        self.class_ctxs.pop();
    }

    fn method(&mut self) {
        self.consume(TokenType::Identifier, "Expect method name.");
        let name_in = self.make_identifier();

        let function_type = if self.tin.pre.description == "init" {
            FunctionType::Initializer
        } else {
            FunctionType::Method
        };

        self.function(function_type);
        self.emit_instruction(Instruction::Method(name_in));
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
        let ctx = &mut cctx!(self);
        if ctx.stack_sim.scope_depth > 0 {
            ctx.stack_sim.mark_initialized();
        } else {
            self.emit_instruction(Instruction::DefineGlobal(global));
        }
    }

    fn parse_variable(&mut self, msg: &str) -> ConstantIndex {
        self.consume(TokenType::Identifier, msg);

        self.declare_variable();
        if cctx!(self).stack_sim.scope_depth > 0 {
            return 0;
        }

        self.make_identifier()
    }

    fn declare_variable(&mut self) {
        let ctx = &mut cctx!(self);
        if ctx.stack_sim.scope_depth == 0 {
            return;
        }

        for (i, local) in ctx.stack_sim.locals.iter().enumerate().rev() {
            if local.depth != -1 && local.depth < ctx.stack_sim.scope_depth {
                break;
            }

            if local.name.description == self.tin.pre.description {
                ctx.errh.error_at(
                    &self.tin.pre,
                    "Already variable with this name in this scope.",
                );
            }
        }

        self.add_local();
    }

    fn make_identifier_from_name(&mut self, name: &str) -> ConstantIndex {
        let lox_str = self.heap.intern_string(name);
        Self::make_constant(&mut cctx!(self), Value::String(lox_str), &self.tin)
    }

    fn make_identifier(&mut self) -> ConstantIndex {
        self.make_identifier_from_name(self.tin.pre.description)
    }

    fn return_statement(&mut self) {
        if let FunctionType::Script = cctx!(self).function_type {
            cctx!(self)
                .errh
                .error_at_previous(&self.tin, "Can't return from top-level code.");
        }

        if self.match_tt(TokenType::SemiColon) {
            self.emit_return();
        } else {
            if cctx!(self).function_type == FunctionType::Initializer {
                self.error_at_previous("Can't return a value from an initializer.");
            }

            self.expression();
            self.consume(TokenType::SemiColon, "Expect ';' after return value.");
            self.emit_instruction(Instruction::Return);
        }
    }

    pub fn statement(&mut self) {
        if self.match_tt(TokenType::Print) {
            self.print_statement();
        } else if self.match_tt(TokenType::If) {
            self.if_statement();
        } else if self.match_tt(TokenType::Return) {
            self.return_statement();
        } else if self.match_tt(TokenType::While) {
            self.while_statement();
        } else if self.match_tt(TokenType::For) {
            self.for_statement();
        } else if self.match_tt(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
            self.consume(TokenType::SemiColon, "Expect ';' after value.");
            self.emit_pop();
        }
    }

    fn for_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.");
        self.begin_scope();

        if self.match_tt(TokenType::SemiColon) {
            // no initializer
        } else if self.match_tt(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let exit_jump;

        let condition_start = cchunk!(self).next_byte_index();
        let mut post_body = condition_start;
        if self.match_tt(TokenType::SemiColon) {
            exit_jump = None;
        } else {
            self.expression();
            self.consume(TokenType::SemiColon, "Expect ';' after loop condition.");

            exit_jump = Some(self.emit_jump(Instruction::jump_if_false_placeholder()));
            self.emit_pop();
        }
        let body_start_patch_loc = self.emit_jump(Instruction::jump_placeholder());

        if !self.match_tt(TokenType::RightParen) {
            post_body = cchunk!(self).next_byte_index();

            self.expression();
            self.emit_pop();

            self.consume(TokenType::RightParen, "Expect ')' after for clauses.");
            self.emit_back_jump(condition_start);
        }

        self.patch_fwd_jump(body_start_patch_loc);
        self.statement();

        self.emit_back_jump(post_body);

        if let Some(exit_jump) = exit_jump {
            self.patch_fwd_jump(exit_jump);
            self.emit_pop();
        }

        self.end_scope();
    }

    pub fn while_statement(&mut self) {
        let loop_jump = cchunk!(self).next_byte_index();
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let exit_jump = self.emit_jump(Instruction::jump_if_false_placeholder());
        self.emit_pop();

        self.statement();
        self.emit_back_jump(loop_jump);
        self.patch_fwd_jump(exit_jump);
        self.emit_pop();
    }

    fn emit_back_jump(&mut self, jump_index: usize) {
        let offset: Result<ByteCodeOffset, _> =
            (cchunk!(self).next_byte_index() - jump_index).try_into();

        if let Ok(offset) = offset {
            self.emit_instruction(Instruction::JumpBack(offset));
        } else {
            cctx!(self)
                .errh
                .error_at_previous(&self.tin, "Loop body too large.");
            // self.emit_instruction(Instruction::JumpBack(0));
        }
    }

    pub fn if_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let patch_loc = self.emit_jump(Instruction::jump_if_false_placeholder());

        // Pop if condition expression from stack.
        self.emit_pop();
        self.statement();
        // Jump to avoid potential else block bytecode coming up next.
        let else_patch_loc = self.emit_jump(Instruction::jump_placeholder());

        self.patch_fwd_jump(patch_loc);

        if self.match_tt(TokenType::Else) {
            // Pop if condition expression from stack.
            // The previously written pop op in this function won't work since it's in the if then block.
            self.emit_pop();

            self.statement();
        }
        self.patch_fwd_jump(else_patch_loc);
    }

    fn patch_fwd_jump(&mut self, patch_loc: usize) {
        let patch: Result<ByteCodeOffset, _> =
            (cchunk!(self).next_byte_index() - patch_loc).try_into();

        if let Ok(patch) = patch {
            cchunk!(self)
                // + 1 ensures that the ByteCodeIndex is written into the jump offset
                // not overrwriting in Instr Opcode
                .patch_bytecode_index(patch_loc + 1, patch as ByteCodeOffset);
        } else {
            cctx!(self)
                .errh
                .error_at_previous(&self.tin, "Too much code to jump over.");
        }
    }

    fn emit_jump(&mut self, instr: Instruction) -> usize {
        let patch_index = cchunk!(self).next_byte_index();
        self.emit_instruction(instr);

        patch_index
    }

    fn begin_scope(&mut self) {
        cctx!(self).stack_sim.begin_scope();
    }

    fn end_scope(&mut self) {
        let ctx = &mut cctx!(self);
        ctx.stack_sim.end_scope();

        for i in (0..ctx.stack_sim.size()).rev() {
            let local = &ctx.stack_sim.locals[i];
            if ctx.stack_sim.scope_depth < local.depth {
                let local = ctx.stack_sim.locals.pop().unwrap();
                if local.captured {
                    ctx.emit_instruction(Instruction::CloseUpvalue, &self.tin);
                } else {
                    ctx.emit_pop(&self.tin);
                }
            } else {
                break;
            }
        }
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
            self.advance();
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
    fn new(name: &'static str) -> Self {
        let mut locals = Vec::with_capacity(LOCALS_MAX_CAPACITY);

        let token = Token {
            line: 0,
            kind: TokenType::Identifier,
            description: name,
        };
        locals.push(Local::new(token, 0));

        Self {
            locals,
            scope_depth: 0,
        }
    }

    fn add_local(&mut self, token: Token<'a>) {
        self.locals.push(Local::new(token, -1));
    }

    fn mark_initialized(&mut self) {
        if self.scope_depth == 0 {
            return;
        }

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
    captured: bool,
}

impl<'a> Local<'a> {
    fn new(token: Token<'a>, depth: isize) -> Self {
        Self {
            name: token,
            depth,
            captured: false,
        }
    }
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

struct CompilerContext<'a> {
    function: LoxFun,
    upvalues: Vec<UpvalueSim>,
    function_type: FunctionType,
    stack_sim: StackSim<'a>,
    errh: ErrorHandler,
}

impl CompilerContext<'_> {
    fn new(function_type: FunctionType, name: Gc<LoxStr>) -> Self {
        let this_name = if function_type == FunctionType::Method
            || function_type == FunctionType::Initializer
        {
            "this"
        } else {
            ""
        };

        Self {
            function: LoxFun::new(name),
            function_type,
            stack_sim: StackSim::new(this_name),
            errh: ErrorHandler {
                had_error: false,
                panic_mode: false,
            },
            upvalues: Vec::new(),
        }
    }

    fn resolve_local(&mut self, cursor: &TokenCursor, name: &str) -> Option<StackIndex> {
        for (i, local) in self.stack_sim.locals.iter().enumerate().rev() {
            if local.name.description == name {
                if local.depth == -1 {
                    self.errh.error_at_previous(
                        cursor,
                        "Can't read local variable in its own initializer.",
                    );
                }

                return Some(i as u8);
            }
        }

        None
    }

    fn emit_instruction(&mut self, instr: Instruction, cursor: &TokenCursor) {
        self.function.chunk.add_instruction(instr, cursor.pre.line);
    }

    fn emit_pop(&mut self, cursor: &TokenCursor) {
        self.emit_instruction(Instruction::Pop, cursor);
    }
}

struct ClassContext<'a> {
    name: Token<'a>,
    has_superclass: bool
}

impl<'a> ClassContext<'a> {
    fn new(token: &Token<'a>) -> Self {
        Self {
            name: token.clone(),
            has_superclass: false,
        }
    }
}
