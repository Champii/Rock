use crate::parser::engine::tests::common::*;
use crate::parser::engine::{many, Parser};

#[test]
fn test_many_zero_matches() {
    let ctx = make_ctx("foo");
    let mut parser = many(number_parser());

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output.len(), 0);
    assert_eq!(rest.len(), 1); // Token not consumed
}
