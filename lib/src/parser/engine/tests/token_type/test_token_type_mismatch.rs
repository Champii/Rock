use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_token_type_mismatch() {
    let ctx = make_ctx("=");
    let mut parser = TokenType::Arrow;

    let result = parser.process(ctx);
    assert!(result.is_err());
}
