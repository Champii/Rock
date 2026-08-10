use crate::ast::*;
use crate::lexer::Span;
use crate::parser::items::*;
use crate::parser::*;
use crate::Config;

#[test]
fn test_parse_assignment_complex() {
    let input = "a.b[2].c = 1";
    let tokens = lex_test(input);
    let config = Config::default();

    let (rest, statement) = statement.process(ParseCtx::from(&tokens, &config)).unwrap();

    assert_eq!(
        statement,
        Statement::Assignment(Assignment {
            lhs: AssignmentLHS::Expression(UnaryExpr::PrimaryExpr(PrimaryExpr {
                operand: Operand::Ident(IdentifierPath {
                    path: vec![IdentOrType::Ident(Ident {
                        name: "a".to_string(),
                        span: Span::default(),
                    })],
                }),
                secondaries: Some(vec![
                    SecondaryExpr::Dot(IdentOrNumber::Ident(Ident {
                        name: "b".to_string(),
                        span: Span::default(),
                    })),
                    SecondaryExpr::Indice(Box::new(Expression::UnaryExpr(UnaryExpr::PrimaryExpr(
                        PrimaryExpr {
                            operand: Operand::Literal(Literal {
                                kind: crate::ast::LiteralKind::Number(2),
                                span: Span::default(),
                            }),
                            secondaries: None,
                            type_annotation: None,
                        }
                    ))),),
                    SecondaryExpr::Dot(IdentOrNumber::Ident(Ident {
                        name: "c".to_string(),
                        span: Span::default(),
                    })),
                ]),
                type_annotation: None,
            })),
            rhs: Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
                operand: Operand::Literal(Literal {
                    kind: crate::ast::LiteralKind::Number(1),
                    span: Span::default(),
                }),
                secondaries: None,
                type_annotation: None,
            })),
        })
    );

    assert_eq!(rest.len(), 0);
}
