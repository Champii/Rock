use crate::parser::engine::tests::common::*;

#[test]
fn test_parse_ctx_consume() {
    let ctx = make_ctx("foo bar");

    let result = ctx.consume();
    assert!(result.is_ok());

    let (new_ctx, token) = result.unwrap();
    assert_eq!(new_ctx.len(), 1);
    assert!(matches!(
        token.token_type,
        crate::lexer::TokenType::Ident(_)
    ));
}
