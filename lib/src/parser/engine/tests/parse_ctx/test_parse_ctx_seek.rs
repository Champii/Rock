use crate::parser::engine::tests::common::*;

#[test]
fn test_parse_ctx_seek() {
    let ctx = make_ctx("foo bar");

    let result = ctx.seek();
    assert!(result.is_ok());

    let token = result.unwrap();
    assert!(matches!(
        token.token_type,
        crate::lexer::TokenType::Ident(_)
    ));

    // Seek should not consume the token
    assert_eq!(ctx.len(), 2);
}
