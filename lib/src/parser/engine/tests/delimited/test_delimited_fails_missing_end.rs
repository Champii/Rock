use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::{delimited, Parser};

#[test]
fn test_delimited_fails_missing_end() {
    let ctx = make_ctx("( foo");
    let mut parser = delimited(TokenType::OpenParen, ident_parser(), TokenType::CloseParen);

    let result = parser.process(ctx);
    assert!(result.is_err());
}
