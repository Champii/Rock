use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_opt_returns_some() {
    let ctx = make_ctx("( foo");
    let mut parser = TokenType::OpenParen.opt();

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert!(output.is_some());
    assert_eq!(output.unwrap(), TokenType::OpenParen);
    assert_eq!(rest.len(), 1);
}
