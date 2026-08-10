use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::{delimited, Parser};

#[test]
fn test_delimited_returns_middle() {
    let ctx = make_ctx("( \"hello\" )");
    let mut parser = delimited(TokenType::OpenParen, string_parser(), TokenType::CloseParen);

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    // Should return only the middle value, not the delimiters
    assert!(matches!(output, crate::lexer::TokenType::String(_)));
    assert_eq!(rest.len(), 0);
}
