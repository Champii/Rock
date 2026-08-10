use crate::parser::engine::tests::common::*;
use crate::parser::engine::Parser;

#[test]
fn test_parser_trait_opt() {
    let ctx = make_ctx("foo");
    let mut parser = ident_parser().opt();

    let result = parser.process(ctx);
    assert!(result.is_ok());

    let (_rest, output) = result.unwrap();
    assert!(matches!(output, Some(crate::lexer::TokenType::Ident(_))));
}
