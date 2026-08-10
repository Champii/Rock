use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_map_chain() {
    let ctx = make_ctx("foo");
    let mut parser = ident_parser().map(|_| 10).map(|x| x * 2).map(|x| x + 5);

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output, 25);
    assert_eq!(rest.len(), 0);
}
