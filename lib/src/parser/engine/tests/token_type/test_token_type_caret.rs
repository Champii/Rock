use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_token_type_caret() {
    let ctx = make_ctx("^");
    let mut parser = TokenType::Caret;

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output, TokenType::Caret);
    assert_eq!(rest.len(), 0);
}
