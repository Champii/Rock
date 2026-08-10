use crate::ast::*;
use crate::parser::items::tests::common::*;

#[test]
fn test_parse_number() {
    let input = "123";
    let literal = parse_literal(input);

    assert_eq!(literal.kind, LiteralKind::Number(123));
}
