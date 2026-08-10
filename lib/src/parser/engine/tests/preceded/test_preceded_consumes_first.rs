use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::{preceded, Parser};

#[test]
fn test_preceded_consumes_first() {
    let ctx = make_ctx("-> foo bar");
    let mut parser = preceded(TokenType::Arrow, ident_parser());

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (rest, output) = result.unwrap();
    assert!(matches!(output, crate::lexer::TokenType::Ident(_)));
    // Should have consumed both arrow and first ident
    assert_eq!(rest.len(), 1);
}
