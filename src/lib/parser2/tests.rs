use nom::Finish;

use super::*;

#[cfg(test)]
mod parse_literal {
    use super::*;

    #[test]
    fn bool() {
        let input = Parser::new_extra("true", ParserCtx::new(PathBuf::new()));

        let (_rest, num_parsed) = parse_literal(input).finish().unwrap();

        assert!(matches!(num_parsed.kind, LiteralKind::Bool(true)));
    }

    #[test]
    fn number() {
        let input = Parser::new_extra("42", ParserCtx::new(PathBuf::new()));

        let (_rest, num_parsed) = parse_literal(input).finish().unwrap();

        assert!(matches!(num_parsed.kind, LiteralKind::Number(42)));
    }

    #[test]
    fn float() {
        let input = Parser::new_extra("42.42", ParserCtx::new(PathBuf::new()));

        let (_rest, num_parsed) = parse_literal(input).finish().unwrap();

        assert!(matches!(num_parsed.kind, LiteralKind::Float(f) if f == 42.42));
    }
}

#[cfg(test)]
mod parse_bool {
    use super::*;

    #[test]
    fn r#true() {
        let input = Parser::new_extra("true", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_bool(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Bool(true)));
    }

    #[test]
    fn r#false() {
        let input = Parser::new_extra("false", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_bool(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Bool(false)));
    }

    #[test]
    fn invalid() {
        let input = Parser::new_extra("atrue", ParserCtx::new(PathBuf::new()));

        assert!(parse_bool(input).finish().is_err());
    }
}

#[cfg(test)]
mod parse_float {
    use super::*;

    #[test]
    fn valid_with_last_part() {
        let input = Parser::new_extra("42.42", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_float(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Float(f) if f == 42.42));
    }

    #[test]
    fn valid_no_last_part() {
        let input = Parser::new_extra("42.", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_float(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Float(f) if f == 42.0));
    }

    #[test]
    fn invalid() {
        let input = Parser::new_extra("a42.", ParserCtx::new(PathBuf::new()));

        assert!(parse_float(input).finish().is_err());
    }
}

#[cfg(test)]
mod parse_number {
    use super::*;

    #[test]
    fn valid() {
        let input = Parser::new_extra("42", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_number(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Number(42)));
    }

    #[test]
    fn invalid() {
        let input = Parser::new_extra("a42", ParserCtx::new(PathBuf::new()));

        assert!(parse_number(input).finish().is_err());
    }
}
/*
#[cfg(test)]
mod parse_string {
    use super::*;

    #[test]
    fn valid() {
        let input = Parser::new_extra("\"hello\"", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_string(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::String(ref s) if s == "hello"));
    }

    #[test]
    fn invalid() {
        let input = Parser::new_extra("\"hello", ParserCtx::new(PathBuf::new()));

        assert!(parse_string(input).finish().is_err());
    }
} */

#[cfg(test)]
mod parse_signature {
    use super::*;

    #[test]
    fn valid_1_arg() {
        let input = Parser::new_extra("Int64", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_signature(input).finish().unwrap();

        assert_eq!(parsed.arguments, vec![]);
        assert_eq!(parsed.ret, Box::new(Type::int64()));
    }

    fn valid_2_arg() {
        let input = Parser::new_extra("Int64 -> Int64", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_signature(input).finish().unwrap();

        assert_eq!(parsed.arguments, vec![Type::int64()]);
        assert_eq!(parsed.ret, Box::new(Type::int64()));
    }
}

#[cfg(test)]
mod parse_type {
    use super::*;

    #[test]
    fn valid() {
        let input = Parser::new_extra("Int64", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_type(input).finish().unwrap();

        assert_eq!(parsed, Type::int64());
    }

    #[test]
    fn invalid() {
        let input = Parser::new_extra("int64", ParserCtx::new(PathBuf::new()));

        assert!(parse_type(input).finish().is_err());
    }
}

#[cfg(test)]
mod parse_infix_op {
    use super::*;

    #[test]
    fn valid() {
        let input = Parser::new_extra("infix + 5", ParserCtx::new(PathBuf::new()));

        let (rest, parsed) = parse_infix(input).finish().unwrap();

        matches!(parsed, TopLevel::Infix(op, 5));

        let operators = HashMap::from([("+".to_string(), 5)]);
        assert_eq!(rest.extra.operators_list, operators);
    }
}

#[cfg(test)]
mod parse_operator {
    use super::*;

    #[test]
    fn valid() {
        let operators = HashMap::from([("+".to_string(), 5)]);

        let input = Parser::new_extra(
            "+",
            ParserCtx::new_with_operators(PathBuf::new(), operators),
        );

        let (rest, parsed) = parse_operator(input).finish().unwrap();

        assert_eq!(
            parsed,
            Operator(Identifier {
                name: String::from("+"),
                node_id: 0,
            })
        );
    }
}

#[cfg(test)]
mod parse_identifier {
    use super::*;

    #[test]
    fn valid() {
        let input = Parser::new_extra("foo", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_identifier(input).finish().unwrap();

        assert_eq!(
            parsed,
            Identifier {
                name: String::from("foo"),
                node_id: 0,
            }
        );
    }
}

#[cfg(test)]
mod parse_identifier_path {
    use super::*;

    #[test]
    fn valid() {
        let input = Parser::new_extra("foo::bar", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_identifier_path(input).finish().unwrap();

        assert_eq!(
            parsed,
            IdentifierPath {
                path: vec![
                    Identifier {
                        name: String::from("foo"),
                        node_id: 0,
                    },
                    Identifier {
                        name: String::from("bar"),
                        node_id: 0,
                    },
                ],
            }
        );
    }
}

#[cfg(test)]
mod parse_operand {
    use super::*;

    #[test]
    fn valid_literal() {
        let input = Parser::new_extra("42", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_operand(input).finish().unwrap();

        matches!(
            parsed,
            Operand::Literal(Literal {
                kind: LiteralKind::Number(42),
                node_id: 0,
            })
        );
    }

    #[test]
    fn valid_identifier_path() {
        let input = Parser::new_extra("foo::bar", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_operand(input).finish().unwrap();

        matches!(parsed, Operand::Identifier(IdentifierPath { path }));
    }

    #[test]
    fn valid_expression() {
        let input = Parser::new_extra("(3)", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_operand(input).finish().unwrap();

        matches!(parsed, Operand::Expression(expr));
    }
}

#[cfg(test)]
mod parse_expression {
    use super::*;

    #[test]
    fn valid_unary() {
        let input = Parser::new_extra("3", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_expression(input).finish().unwrap();

        matches!(parsed, Expression::UnaryExpr(_));
    }

    #[test]
    fn valid_binary() {
        let input = Parser::new_extra("3 + 4", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_expression(input).finish().unwrap();

        matches!(parsed, Expression::BinopExpr(_, _, _));
    }
}

#[cfg(test)]
mod parse_fn_decl {
    use super::*;

    #[test]
    fn valid() {
        let input = Parser::new_extra("toto a b = a + b", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_fn(input).finish().unwrap();

        let expected = FunctionDecl {
            name: Identifier {
                name: String::from("toto"),
                node_id: 0,
            },
            arguments: vec![
                Identifier {
                    name: String::from("a"),
                    node_id: 0,
                },
                Identifier {
                    name: String::from("b"),
                    node_id: 0,
                },
            ],
            body: Body {
                stmts: vec![Statement::Expression(Box::new(Expression::BinopExpr(
                    UnaryExpr::PrimaryExpr(PrimaryExpr {
                        op: Operand::Identifier(IdentifierPath {
                            path: vec![Identifier {
                                name: String::from("a"),
                                node_id: 0,
                            }],
                        }),
                        node_id: 0,
                        secondaries: None,
                    }),
                    Operator(Identifier {
                        name: String::from("+"),
                        node_id: 0,
                    }),
                    Box::new(Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
                        op: Operand::Identifier(IdentifierPath {
                            path: vec![Identifier {
                                name: String::from("b"),
                                node_id: 0,
                            }],
                        }),
                        node_id: 0,
                        secondaries: None,
                    }))),
                )))],
            },
            signature: FuncType {
                ret: Box::new(Type::forall("c")),
                arguments: vec![Type::forall("a"), Type::forall("b")],
            },
        };

        assert_eq!(parsed.name, expected.name);
        assert_eq!(parsed.arguments, expected.arguments);
        // assert_eq!(parsed.body, expected.body);
        assert_eq!(parsed.signature, expected.signature);
    }
}
