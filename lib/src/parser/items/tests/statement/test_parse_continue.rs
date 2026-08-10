use crate::ast::*;
use crate::lexer::Span;
use crate::parser::items::*;
use crate::parser::*;
use crate::Config;

#[test]
fn test_parse_continue() {
    let input = "continue 1";
    let tokens = lex_test(input);
    let config = Config::default();

    let (rest, statement) = statement.process(ParseCtx::from(&tokens, &config)).unwrap();

    assert_eq!(
        statement,
        Statement::Continue(Some(Expression::UnaryExpr(UnaryExpr::PrimaryExpr(
            PrimaryExpr {
                operand: Operand::Literal(Literal {
                    kind: crate::ast::LiteralKind::Number(1),
                    span: Span::default(),
                }),
                secondaries: None,
                type_annotation: None,
            }
        ))))
    );

    assert_eq!(rest.len(), 0);
}
