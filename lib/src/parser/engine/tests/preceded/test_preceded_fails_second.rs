use crate::lexer::TokenType;
use crate::parser::engine::tests::common::*;
use crate::parser::engine::{preceded, Parser};

#[test]
fn test_preceded_fails_second() {
    let ctx = make_ctx("= 42");
    let mut parser = preceded(TokenType::Equal, ident_parser());

    let result = parser.process(ctx);
    assert!(result.is_err());
}
