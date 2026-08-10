use crate::ast::*;
use crate::lexer::Span;
use crate::parser::items::*;
use crate::parser::*;
use crate::Config;

#[test]
fn test_parse_patter_with_binding() {
    let input = "a @ 1";
    let tokens = lex_test(input);
    let config = Config::default();

    let (rest, pattern) = pattern.process(ParseCtx::from(&tokens, &config)).unwrap();

    assert_eq!(
        pattern,
        Pattern {
            binding: Some(Ident {
                name: "a".to_string(),
                span: Span::default()
            }),
            kind: PatternKind::Literal(Literal {
                kind: LiteralKind::Number(1),
                span: Span::default()
            })
        }
    );

    assert_eq!(rest.len(), 0);
}
