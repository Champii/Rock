use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::{separated1, Parser};

#[test]
fn test_separated1_empty_fails() {
    let ctx = make_ctx("foo");
    let mut parser = separated1(number_parser(), TokenType::Coma);

    let result = parser.process(ctx);
    assert!(result.is_err());
}
