use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_opt_fails_returns_none() {
    let ctx = make_ctx("42");
    let mut parser = ident_parser().opt();

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output, None);
    assert_eq!(rest.len(), 1); // Token not consumed
}
