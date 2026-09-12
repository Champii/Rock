use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_map_with_closure() {
    let ctx = make_ctx("foo");
    let mut parser = ident_parser().map(|token_type| match token_type {
        TokenType::Ident(s) => s,
        _ => panic!("Expected Ident"),
    });

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output, "foo");
    assert_eq!(rest.len(), 0);
}
