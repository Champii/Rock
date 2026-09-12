use crate::parser::engine::tests::common::*;
use crate::parser::engine::{many, Parser};

#[test]
fn test_many_multiple_matches() {
    let ctx = make_ctx("foo bar baz");
    let mut parser = many(ident_parser());

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert_eq!(output.len(), 3);
    assert!(matches!(output[0], crate::lexer::TokenType::Ident(_)));
    assert!(matches!(output[1], crate::lexer::TokenType::Ident(_)));
    assert!(matches!(output[2], crate::lexer::TokenType::Ident(_)));
    assert_eq!(rest.len(), 0);
}
