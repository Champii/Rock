use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_map_to_constant() {
    let ctx = make_ctx("42");
    let mut parser = number_parser().map(|_| "constant");

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output, "constant");
    assert_eq!(rest.len(), 0);
}
