use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_and_first_fails() {
    let ctx = make_ctx("42 foo");
    let mut parser = ident_parser().and(number_parser());

    let result = parser.process(ctx);
    assert!(result.is_err());
}
