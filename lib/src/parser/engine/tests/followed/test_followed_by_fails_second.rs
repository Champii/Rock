use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_followed_by_fails_second() {
    let ctx = make_ctx("foo 42");
    let mut parser = ident_parser().followed_by(TokenType::Equal);

    let result = parser.process(ctx);
    assert!(result.is_err());
}
