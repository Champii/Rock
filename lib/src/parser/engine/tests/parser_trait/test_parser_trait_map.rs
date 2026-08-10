use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_parser_trait_map() {
    let ctx = make_ctx("foo");
    let mut parser = ident_parser().map(|_| "mapped");

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (_rest, output) = result.unwrap();
    assert_eq!(output, "mapped");
}
