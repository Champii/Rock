use crate::ast::Expression;
use crate::parser::engine::{ParseCtx, Parser};
use crate::parser::items::expression;
use crate::parser::lex_test;
use crate::Config;

fn parse_range(source: &str) -> crate::ast::RangeExpr {
    let tokens = lex_test(source);
    let config = Config::default();
    let (rest, expression) = expression
        .process(ParseCtx::from(&tokens, &config))
        .unwrap();
    assert_eq!(rest.len(), 0, "range parser left trailing tokens");
    let Expression::Range(range) = expression else {
        panic!("expected range expression for {source:?}, got {expression:?}");
    };
    range
}

#[test]
fn parses_all_native_range_forms() {
    for (source, has_start, has_end, inclusive) in [
        ("1..4", true, true, false),
        ("1..=4", true, true, true),
        ("..4", false, true, false),
        ("..=4", false, true, true),
        ("1..", true, false, false),
        ("..", false, false, false),
    ] {
        let range = parse_range(source);
        assert_eq!(range.start.is_some(), has_start, "source: {source}");
        assert_eq!(range.end.is_some(), has_end, "source: {source}");
        assert_eq!(range.inclusive, inclusive, "source: {source}");
    }
}

#[test]
fn range_endpoints_accept_expressions() {
    let range = parse_range("start + 1..finish * 2");
    assert!(range.start.is_some());
    assert!(range.end.is_some());
}

#[test]
fn inclusive_range_requires_an_end() {
    let tokens = lex_test("1..=");
    let config = Config::default();
    assert!(expression
        .process(ParseCtx::from(&tokens, &config))
        .is_err());
}
