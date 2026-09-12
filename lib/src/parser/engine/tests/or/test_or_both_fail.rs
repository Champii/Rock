use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_or_both_fail() {
    let ctx = make_ctx("\"hello\"");
    let mut parser = ident_parser().or(number_parser());

    let result = parser.process(ctx);
    assert!(result.is_err());
}
