use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::{delimited, Parser};

#[test]
fn test_delimited_parens() {
    let ctx = make_ctx("( foo )");
    let mut parser = delimited(TokenType::OpenParen, ident_parser(), TokenType::CloseParen);

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert!(matches!(output, crate::lexer::TokenType::Ident(_)));
    assert_eq!(rest.len(), 0);
}
