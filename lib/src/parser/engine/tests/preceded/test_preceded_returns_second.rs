use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::{preceded, Parser};

#[test]
fn test_preceded_returns_second() {
    let ctx = make_ctx("= foo");
    let mut parser = preceded(TokenType::Equal, ident_parser());

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert!(matches!(output, crate::lexer::TokenType::Ident(_)));
    assert_eq!(rest.len(), 0);
}
