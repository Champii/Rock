use crate::ast::*;
use crate::parser::items::tests::common::*;

#[test]
fn test_parse_array_empty() {
    let input = "[]";
    let literal = parse_literal(input);

    assert_eq!(literal.kind, LiteralKind::Array(Array { elements: vec![] }));
}
