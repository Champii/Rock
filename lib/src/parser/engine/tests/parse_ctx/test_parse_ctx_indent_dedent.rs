use crate::parser::engine::tests::common::*;

#[test]
fn test_parse_ctx_indent_dedent() {
    let ctx = make_ctx("foo");

    let initial_indent = ctx.indent_level;

    let indented = ctx.indent().unwrap();
    assert_eq!(indented.indent_level, initial_indent + ctx.indent_step);

    let dedented = indented.dedent().unwrap();
    assert_eq!(dedented.indent_level, initial_indent);
}
