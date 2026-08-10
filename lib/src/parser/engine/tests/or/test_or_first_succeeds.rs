use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_or_first_succeeds() {
    let ctx = make_ctx("foo");
    let mut parser = ident_parser().or(number_parser());

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert!(matches!(output, crate::lexer::TokenType::Ident(_)));
    assert_eq!(rest.len(), 0);
}
