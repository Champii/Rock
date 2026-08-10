use crate::ast::*;
use crate::parser::items::tests::common::*;

#[test]
fn test_parse_bool() {
    let input = "true";
    let literal = parse_literal(input);

    assert_eq!(literal.kind, LiteralKind::Bool(true));
}
