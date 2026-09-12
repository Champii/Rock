use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_map_transforms_output() {
    let ctx = make_ctx("foo");
    let mut parser = ident_parser().map(|_| 42);

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output, 42);
    assert_eq!(rest.len(), 0);
}
