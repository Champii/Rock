use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_followed_by_returns_first() {
    let ctx = make_ctx("foo -> bar");
    let mut parser = ident_parser().followed_by(TokenType::Arrow);

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    // Should return the first parser's output
    assert!(matches!(output, crate::lexer::TokenType::Ident(_)));
    // Should consume both tokens
    assert_eq!(rest.len(), 1);
}
