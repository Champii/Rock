use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_and_returns_tuple() {
    let ctx = make_ctx("[ ]");
    let mut parser = TokenType::OpenBracket.and(TokenType::CloseBracket);

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output.0, TokenType::OpenBracket);
    assert_eq!(output.1, TokenType::CloseBracket);
    assert_eq!(rest.len(), 0);
}
