use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::{delimited, Parser};

#[test]
fn test_delimited_brackets() {
    let ctx = make_ctx("[ 42 ]");
    let mut parser = delimited(
        TokenType::OpenBracket,
        number_parser(),
        TokenType::CloseBracket,
    );

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert!(matches!(output, crate::lexer::TokenType::Number(_)));
    assert_eq!(rest.len(), 0);
}
