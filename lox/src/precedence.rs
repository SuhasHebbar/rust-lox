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

pub type ParseFn = Box<dyn Fn(&mut Compiler)>;

fn placeholder_fn(_compiler: &mut Compiler) {
    eprintln!("Call to undefined table entry.");
} 

thread_local!(static PLACEHOLDER_PARSEFN: ParseFn = Box::new(placeholder_fn));

pub struct ParseRule {
    prefix: ParseFn,
    infix: ParseFn,
    precedence: Precedence,
}

impl ParseRule {
    fn new(infix: ParseFn, prefix: ParseFn, precedence: Precedence) -> Self {
        ParseRule {
            infix,
            prefix,
            precedence,
        }
    }
}

// const PLACEHOLDER_PARSERULE: ParseRule =
//     ParseRule::new(PLACEHOLDER_PARSEFN, PLACEHOLDER_PARSEFN, Precedence::None);

// const LEFT_PAREN_RULE: ParseRule = ParseRule::new(
//     Box::new(Compiler::grouping),
//     PLACEHOLDER_PARSEFN,
//     Precedence::None,
// );

// const MINUS_RULE: ParseRule = ParseRule::new(
//     Box::new(Compiler::unary),
//     Box::new(Compiler::binary),
//     Precedence::Term,
// );

// const PLUS_RULE: ParseRule = ParseRule::new(
//     PLACEHOLDER_PARSEFN,
//     Box::new(Compiler::binary),
//     Precedence::Term,
// );

// const SLASH_AND_STAR_RULE: ParseRule = ParseRule::new(
//     PLACEHOLDER_PARSEFN,
//     Box::new(Compiler::binary),
//     Precedence::Factor,
// );

// const NUMBER_RULE: ParseRule = ParseRule::new(
//     Box::new(Compiler::number),
//     PLACEHOLDER_PARSEFN,
//     Precedence::None,
// );

// pub fn parse_rule(token_type: TokenType) -> ParseRule {
//     match token_type {
//         TokenType::LeftParen => LEFT_PAREN_RULE,
//         TokenType::Minus => MINUS_RULE,
//         TokenType::Plus => PLUS_RULE,
//         TokenType::Slash | TokenType::Star => SLASH_AND_STAR_RULE,
//         TokenType::Number => NUMBER_RULE,
//         _ => PLACEHOLDER_PARSERULE,
//     }
// }
