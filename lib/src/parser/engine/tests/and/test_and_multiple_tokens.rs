use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_and_multiple_tokens() {
    let ctx = make_ctx("( foo )");
    let mut parser = TokenType::OpenParen
        .and(ident_parser())
        .and(TokenType::CloseParen);

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, ((lparen, ident), rparen)) = result.unwrap();
    assert_eq!(lparen, TokenType::OpenParen);
    assert!(matches!(ident, crate::lexer::TokenType::Ident(_)));
    assert_eq!(rparen, TokenType::CloseParen);
    assert_eq!(rest.len(), 0);
}
