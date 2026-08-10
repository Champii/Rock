use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::{separated1, Parser};

#[test]
fn test_separated1_one_item() {
    let ctx = make_ctx("42");
    let mut parser = separated1(number_parser(), TokenType::Coma);

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output.len(), 1);
    assert!(matches!(output[0], crate::lexer::TokenType::Number(_)));
    assert_eq!(rest.len(), 0);
}
