use crate::parser::engine::tests::common::*;

#[test]
fn test_parse_ctx_len() {
    let ctx = make_ctx("");
    assert_eq!(ctx.len(), 0);

    let ctx = make_ctx("foo");
    assert_eq!(ctx.len(), 1);

    let ctx = make_ctx("foo bar baz");
    assert_eq!(ctx.len(), 3);
}
