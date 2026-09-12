use crate::parser::engine::tests::common::*;
use crate::parser::engine::{many1, Parser};

#[test]
fn test_many1_one_match() {
    let ctx = make_ctx("42");
    let mut parser = many1(number_parser());

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output.len(), 1);
    assert!(matches!(output[0], crate::lexer::TokenType::Number(_)));
    assert_eq!(rest.len(), 0);
}
