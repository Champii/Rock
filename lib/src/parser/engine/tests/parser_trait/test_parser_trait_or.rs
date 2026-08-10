use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_parser_trait_or() {
    let ctx = make_ctx("foo");
    let mut parser = number_parser().or(ident_parser());

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (_rest, output) = result.unwrap();
    assert!(matches!(output, crate::lexer::TokenType::Ident(_)));
}
