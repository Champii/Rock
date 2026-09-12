use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_token_type_equal() {
    let ctx = make_ctx("=");
    let mut parser = TokenType::Equal;

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output, TokenType::Equal);
    assert_eq!(rest.len(), 0);
}
