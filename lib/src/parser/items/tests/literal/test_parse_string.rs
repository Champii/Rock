use crate::ast::*;
use crate::parser::items::tests::common::*;

#[test]
fn test_parse_string() {
    let input = "\"hello\"";
    let literal = parse_literal(input);

    assert_eq!(literal.kind, LiteralKind::String("hello".to_owned()));
}
