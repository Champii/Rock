use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_token_type_dot() {
    let ctx = make_ctx(".");
    let mut parser = TokenType::Dot;

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output, TokenType::Dot);
    assert_eq!(rest.len(), 0);
}
