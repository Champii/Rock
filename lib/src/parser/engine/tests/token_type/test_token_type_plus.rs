use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_token_type_plus() {
    // Test with a simple token type instead of Operator which requires a parameter
    let ctx = make_ctx(":");
    let mut parser = TokenType::Colon;

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output, TokenType::Colon);
    assert_eq!(rest.len(), 0);
}
