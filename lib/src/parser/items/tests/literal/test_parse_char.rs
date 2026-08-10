use crate::ast::*;
use crate::parser::items::tests::common::*;

#[test]
fn test_parse_char() {
    let input = "'a'";
    let literal = parse_literal(input);

    assert_eq!(literal.kind, LiteralKind::Char("a".to_owned()));
}
