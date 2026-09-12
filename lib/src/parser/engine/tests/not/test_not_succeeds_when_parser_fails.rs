use crate::parser::engine::tests::common::*;
use crate::parser::engine::{not, Parser};

#[test]
fn test_not_succeeds_when_parser_fails() {
    let ctx = make_ctx("foo");
    let mut parser = not(number_parser());

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, _output) = result.unwrap();
    // Should not consume any tokens
    assert_eq!(rest.len(), 1);
}
