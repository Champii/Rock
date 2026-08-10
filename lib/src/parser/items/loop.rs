use crate::{
    lexer::TokenType,
    parser::{engine::*, Loop},
};

use super::{block, disallow_multiline_fn_call, expression, parse_condition, pattern};

pub fn r#loop(stream: Input) -> IResult<Loop> {
    raw_loop.or(r#for).or(r#while).process(stream)
}

pub fn raw_loop(stream: Input) -> IResult<Loop> {
    (TokenType::Keyword("loop".to_string()), block)
        .map(|(_, body)| Loop::Loop(body))
        .process(stream)
}

pub fn r#while(stream: Input) -> IResult<Loop> {
    (
        TokenType::Keyword("while".to_string()),
        parse_condition,
        block,
    )
        .map(|(_, condition, body)| Loop::While(condition, body))
        .process(stream)
}

pub fn r#for(stream: Input) -> IResult<Loop> {
    (
        TokenType::Keyword("for".to_string()),
        pattern,
        TokenType::Keyword("in".to_string()),
        disallow_multiline_fn_call(expression),
        block,
    )
        .map(|(_, ident, _, expr, body)| Loop::For(ident, expr, body))
        .process(stream)
}
