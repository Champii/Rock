use crate::ast::*;
use crate::parser::items::tests::common::*;

#[test]
fn test_parse_float() {
    let input = "123.456";
    let literal = parse_literal(input);

    assert_eq!(literal.kind, LiteralKind::Float(123.456));
}
