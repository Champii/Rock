use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_or_second_succeeds() {
    let ctx = make_ctx("42");
    let mut parser = ident_parser().or(number_parser());

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert!(matches!(output, crate::lexer::TokenType::Number(_)));
    assert_eq!(rest.len(), 0);
}
