use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_opt_doesnt_consume_on_failure() {
    let ctx = make_ctx("foo");
    let mut parser = number_parser().opt();

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output, None);
    // Should not consume the token
    assert_eq!(rest.len(), 1);
}
