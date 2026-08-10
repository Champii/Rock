use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_parser_trait_followed_by() {
    let ctx = make_ctx("foo =");
    let mut parser = ident_parser().followed_by(TokenType::Equal);

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (_rest, output) = result.unwrap();
    assert!(matches!(output, crate::lexer::TokenType::Ident(_)));
}
