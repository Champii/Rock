use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_token_type_string() {
    let ctx = make_ctx("\"hello\"");
    let mut parser = string_parser();

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert!(matches!(output, crate::lexer::TokenType::String(_)));
    assert_eq!(rest.len(), 0);
}
