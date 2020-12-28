use crate::{compiler::Compiler, scanner::TokenType};

#[derive(PartialEq, PartialOrd)]
pub enum Precedence {
    None,

    // =
    Assignment,

    // or
    Or,

    // and
    And,

    // ==, !=
    Equality,

    // <, >, <=, >=
    Comparison,

    // +, -
    Term,

    // *, /
    Factor,

    // !, -
    Unary,

    // ., ()
    Call,

    Primary,
}


impl Precedence {
    pub fn next_greater(&self) -> Self {
        use Precedence::*;
        match self {
            None => Assignment,
            Assignment => Or,
            Or => And,
            And => Equality,
            Equality => Comparison,
            Comparison => Term,
            Term => Factor,
            Factor => Term,
            Unary => Call,
            Call => Primary,
            Primary => panic!("There is not precdence greater than Precedence::Primary."),
        }
    }
}

pub type ParseFn = Option<&'static dyn Fn(&mut Compiler)>;

pub struct ParseRule {
    pub prefix: ParseFn,
    pub infix: ParseFn,
    pub precedence: Precedence,
}

const PLACEHOLDER_PARSERULE: ParseRule =
    ParseRule {infix: None, prefix: None, precedence: Precedence::None };


const LEFT_PAREN_RULE: ParseRule = ParseRule {
    prefix: Some(&|this: &mut Compiler| this.grouping()),
    infix: None,
    precedence: Precedence::None
};

const MINUS_RULE: ParseRule = ParseRule {
    prefix: Some(&|this: &mut Compiler| this.unary()),
    infix: Some(&|this: &mut Compiler| this.binary()),
    precedence: Precedence::Term
};

const PLUS_RULE: ParseRule = ParseRule {
    prefix: None,
    infix: Some(&|this: &mut Compiler| this.binary()),
    precedence: Precedence::Term,
};

const SLASH_AND_STAR_RULE: ParseRule = ParseRule {
    prefix: None,
    infix: Some(&|this: &mut Compiler| this.binary()),
    precedence: Precedence::Factor,
};

const NUMBER_RULE: ParseRule = ParseRule {
    prefix: Some(&|this: &mut Compiler| this.number()),
    infix: None,
    precedence: Precedence::None,
};

pub fn parse_rule(token_type: TokenType) -> &'static ParseRule {
    match token_type {
        TokenType::LeftParen => &LEFT_PAREN_RULE,
        TokenType::Minus => &MINUS_RULE,
        TokenType::Plus => &PLUS_RULE,
        TokenType::Slash | TokenType::Star => &SLASH_AND_STAR_RULE,
        TokenType::Number => &NUMBER_RULE,
        _ => &PLACEHOLDER_PARSERULE,
    }
}
