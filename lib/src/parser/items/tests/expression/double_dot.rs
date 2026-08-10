use crate::ast::*;
use crate::lexer::Span;
use crate::parser::items::*;
use crate::parser::*;
use crate::Config;
use std::path::PathBuf;

#[test]
fn double_dot() {
    let input = "foo bar ..baz";
    let tokens = lex_test(input);
    let config = Config::default();

    let (rest, expression) = expression
        .process(ParseCtx::from(&tokens, &config))
        .unwrap();

    assert_eq!(
        expression,
        Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
            operand: Operand::Ident(IdentifierPath {
                path: vec![IdentOrType::Ident(Ident {
                    name: "foo".to_string(),
                    span: Span {
                        start: 0,
                        end: 3,
                        file_path: PathBuf::default(),
                    },
                })],
            }),
            secondaries: Some(vec![
                SecondaryExpr::Arguments(vec![Argument {
                    arg: Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
                        operand: Operand::Ident(IdentifierPath {
                            path: vec![IdentOrType::Ident(Ident {
                                name: "bar".to_string(),
                                span: Span {
                                    start: 4,
                                    end: 7,
                                    file_path: PathBuf::default(),
                                },
                            })],
                        }),
                        secondaries: None,
                        type_annotation: None,
                    }))
                }]),
                SecondaryExpr::DoubleDot(IdentOrNumber::Ident(Ident {
                    name: "baz".to_string(),
                    span: Span {
                        start: 10,
                        end: 13,
                        file_path: PathBuf::default(),
                    },
                })),
            ]),
            type_annotation: None,
        })),
    );
    assert_eq!(rest.len(), 0);
}
