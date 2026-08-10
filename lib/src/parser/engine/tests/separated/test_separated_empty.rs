use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::{separated, Parser};

#[test]
fn test_separated_empty() {
    let ctx = make_ctx("foo");
    let mut parser = separated(number_parser(), TokenType::Coma);

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output.len(), 0);
    assert_eq!(rest.len(), 1); // Token not consumed
}
