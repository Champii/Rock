use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_and_both_succeed() {
    let ctx = make_ctx("foo 42");
    let mut parser = ident_parser().and(number_parser());

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, (first, second)) = result.unwrap();
    assert!(matches!(first, crate::lexer::TokenType::Ident(_)));
    assert!(matches!(second, crate::lexer::TokenType::Number(_)));
    assert_eq!(rest.len(), 0);
}
