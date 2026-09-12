use crate::parser::engine::tests::common::*;

#[test]
fn test_parse_ctx_is_empty() {
    let ctx = make_ctx("");
    assert!(ctx.is_empty());

    let ctx = make_ctx("foo");
    assert!(!ctx.is_empty());
}
