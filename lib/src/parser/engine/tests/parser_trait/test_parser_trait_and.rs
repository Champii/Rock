use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_parser_trait_and() {
    let ctx = make_ctx("foo 42");
    let mut parser = ident_parser().and(number_parser());

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (_rest, (first, second)) = result.unwrap();
    assert!(matches!(first, crate::lexer::TokenType::Ident(_)));
    assert!(matches!(second, crate::lexer::TokenType::Number(_)));
}
