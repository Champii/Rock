use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::{separated1, Parser};

#[test]
fn test_separated1_multiple_items() {
    let ctx = make_ctx("1 , 2 , 3");
    let mut parser = separated1(number_parser(), TokenType::Coma);

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output.len(), 3);
    assert!(matches!(output[0], crate::lexer::TokenType::Number(_)));
    assert!(matches!(output[1], crate::lexer::TokenType::Number(_)));
    assert!(matches!(output[2], crate::lexer::TokenType::Number(_)));
    assert_eq!(rest.len(), 0);
}
