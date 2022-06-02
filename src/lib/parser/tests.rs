use nom::Finish;

use super::*;

#[cfg(test)]
mod parse_literal {
    use super::*;

    #[test]
    fn bool() {
        let input = Parser::new_extra("true", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, num_parsed) = parse_literal(input).finish().unwrap();

        assert!(matches!(num_parsed.kind, LiteralKind::Bool(true)));
    }

    #[test]
    fn number() {
        let input = Parser::new_extra("42", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, num_parsed) = parse_literal(input).finish().unwrap();

        assert!(matches!(num_parsed.kind, LiteralKind::Number(42)));
    }

    #[test]
    fn float() {
        let input = Parser::new_extra("42.42", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, num_parsed) = parse_literal(input).finish().unwrap();

        assert!(matches!(num_parsed.kind, LiteralKind::Float(f) if f == 42.42));
    }
}

#[cfg(test)]
mod parse_bool {
    use super::*;

    #[test]
    fn r#true() {
        let input = Parser::new_extra("true", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, parsed) = parse_bool(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Bool(true)));
    }

    #[test]
    fn r#false() {
        let input = Parser::new_extra("false", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, parsed) = parse_bool(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Bool(false)));
    }

    #[test]
    fn invalid() {
        let input = Parser::new_extra("atrue", ParserCtx::new(PathBuf::new(), Config::default()));

        assert!(parse_bool(input).finish().is_err());
    }
}

#[cfg(test)]
mod parse_float {
    use super::*;

    #[test]
    fn valid_with_last_part() {
        let input = Parser::new_extra("42.42", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, parsed) = parse_float(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Float(f) if f == 42.42));
    }

    #[test]
    fn valid_no_last_part() {
        let input = Parser::new_extra("42.", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, parsed) = parse_float(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Float(f) if f == 42.0));
    }

    #[test]
    fn invalid() {
        let input = Parser::new_extra("a42.", ParserCtx::new(PathBuf::new(), Config::default()));

        assert!(parse_float(input).finish().is_err());
    }
}

#[cfg(test)]
mod parse_number {
    use super::*;

    #[test]
    fn valid() {
        let input = Parser::new_extra("42", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, parsed) = parse_number(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Number(42)));
    }

    #[test]
    fn invalid() {
        let input = Parser::new_extra("a42", ParserCtx::new(PathBuf::new(), Config::default()));

        assert!(parse_number(input).finish().is_err());
    }
}

#[cfg(test)]
mod parse_signature {
    use super::*;

    #[test]
    fn valid_1_arg() {
        let input = Parser::new_extra("Int64", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, parsed) = parse_signature(input).finish().unwrap();

        assert_eq!(parsed.arguments, vec![]);
        assert_eq!(parsed.ret, Box::new(Type::int64()));
    }

    #[test]
    fn valid_2_arg() {
        let input = Parser::new_extra(
            "Int64 -> Int64",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

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
        let input = Parser::new_extra("Int64", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, parsed) = parse_type(input).finish().unwrap();

        assert_eq!(parsed, Type::int64());
    }

    #[test]
    fn valid_for_all() {
        let input = Parser::new_extra("a", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, parsed) = parse_type(input).finish().unwrap();

        assert_eq!(parsed, Type::forall("a"));
    }

    #[test]
    fn invalid() {
        let input = Parser::new_extra("int64", ParserCtx::new(PathBuf::new(), Config::default()));

        assert!(parse_type(input).finish().is_err());
    }
}

#[cfg(test)]
mod parse_infix_op {
    use super::*;

    #[test]
    fn valid() {
        let input = Parser::new_extra(
            "infix + 5",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, parsed) = parse_infix(input).finish().unwrap();

        assert!(matches!(parsed, TopLevel::Infix(_op, 5)));

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
            ParserCtx::new_with_operators(PathBuf::new(), operators, Config::default()),
        );

        let (_rest, parsed) = parse_operator(input).finish().unwrap();

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
        let input = Parser::new_extra("foo", ParserCtx::new(PathBuf::new(), Config::default()));

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
        let input = Parser::new_extra(
            "foo::bar",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

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
        let input = Parser::new_extra("42", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, parsed) = parse_operand(input).finish().unwrap();

        assert!(matches!(
            parsed,
            Operand::Literal(Literal {
                kind: LiteralKind::Number(42),
                node_id: 0,
            })
        ));
    }

    #[test]
    fn valid_identifier_path() {
        let input = Parser::new_extra(
            "foo::bar",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (_rest, parsed) = parse_operand(input).finish().unwrap();

        assert!(matches!(
            parsed,
            Operand::Identifier(IdentifierPath { path: _ })
        ));
    }

    #[test]
    fn valid_expression() {
        let input = Parser::new_extra("(3)", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, parsed) = parse_operand(input).finish().unwrap();

        assert!(matches!(parsed, Operand::Expression(_expr)));
    }
}

#[cfg(test)]
mod parse_expression {
    use super::*;

    #[test]
    fn valid_unary() {
        let input = Parser::new_extra("3", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, parsed) = parse_expression(input).finish().unwrap();

        assert!(matches!(parsed, Expression::UnaryExpr(_)));
    }

    #[test]
    fn valid_binary() {
        let operators = HashMap::from([("+".to_string(), 5)]);

        let input = Parser::new_extra(
            "3 + 4",
            ParserCtx::new_with_operators(PathBuf::new(), operators, Config::default()),
        );

        let (_rest, parsed) = parse_expression(input).finish().unwrap();

        assert!(matches!(parsed, Expression::BinopExpr(_, _, _)));
    }
}
#[cfg(test)]
mod parse_primary {
    use super::*;

    mod arguments {
        use super::*;

        mod parenthesis {
            use super::*;

            #[test]
            fn valid_no_args() {
                let input =
                    Parser::new_extra("foo()", ParserCtx::new(PathBuf::new(), Config::default()));

                let (_rest, parsed) = parse_primary(input).finish().unwrap();

                let secondaries = parsed.secondaries.unwrap();
                assert_eq!(secondaries.len(), 1);

                let args = &secondaries[0];

                match args {
                    SecondaryExpr::Arguments(arr) => assert_eq!(arr.len(), 0),
                    _ => panic!("expected Arguments"),
                }
            }

            #[test]
            fn valid_one_arg() {
                let input =
                    Parser::new_extra("foo(2)", ParserCtx::new(PathBuf::new(), Config::default()));

                let (_rest, parsed) = parse_primary(input).finish().unwrap();

                let secondaries = parsed.secondaries.unwrap();
                assert_eq!(secondaries.len(), 1);

                let args = &secondaries[0];

                match args {
                    SecondaryExpr::Arguments(arr) => assert_eq!(arr.len(), 1),
                    _ => panic!("expected Arguments"),
                }
            }

            #[test]
            fn valid_two_args() {
                let input = Parser::new_extra(
                    "foo(2, 3)",
                    ParserCtx::new(PathBuf::new(), Config::default()),
                );

                let (_rest, parsed) = parse_primary(input).finish().unwrap();

                let secondaries = parsed.secondaries.unwrap();
                assert_eq!(secondaries.len(), 1);

                let args = &secondaries[0];

                match args {
                    SecondaryExpr::Arguments(arr) => assert_eq!(arr.len(), 2),
                    _ => panic!("expected Arguments"),
                }
            }
        }

        mod no_parenthesis {
            use super::*;

            #[test]
            fn valid_one_arg() {
                let input =
                    Parser::new_extra("foo 2", ParserCtx::new(PathBuf::new(), Config::default()));

                let (_rest, parsed) = parse_primary(input).finish().unwrap();

                let secondaries = parsed.secondaries.unwrap();
                assert_eq!(secondaries.len(), 1);

                let args = &secondaries[0];

                match args {
                    SecondaryExpr::Arguments(arr) => assert_eq!(arr.len(), 1),
                    _ => panic!("expected Arguments"),
                }
            }

            #[test]
            fn valid_two_args() {
                let input = Parser::new_extra(
                    "foo 2, 3",
                    ParserCtx::new(PathBuf::new(), Config::default()),
                );

                let (_rest, parsed) = parse_primary(input).finish().unwrap();

                let secondaries = parsed.secondaries.unwrap();
                assert_eq!(secondaries.len(), 1);

                let args = &secondaries[0];

                match args {
                    SecondaryExpr::Arguments(arr) => assert_eq!(arr.len(), 2),
                    _ => panic!("expected Arguments"),
                }
            }
        }
    }

    #[test]
    fn valid_indice() {
        let input = Parser::new_extra("foo[3]", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, parsed) = parse_primary(input).finish().unwrap();

        let secondaries = parsed.secondaries.unwrap();
        assert_eq!(secondaries.len(), 1);

        let args = &secondaries[0];

        match args {
            SecondaryExpr::Indice(_expr) => {}
            _ => panic!("expected indice"),
        }
    }

    #[test]
    fn valid_dot() {
        let input = Parser::new_extra(
            "foo.toto",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (_rest, parsed) = parse_primary(input).finish().unwrap();

        let secondaries = parsed.secondaries.unwrap();
        assert_eq!(secondaries.len(), 1);

        let args = &secondaries[0];

        match args {
            SecondaryExpr::Dot(_expr) => {}
            _ => panic!("expected dot"),
        }
    }

    #[test]
    fn valid_mixed() {
        let input = Parser::new_extra(
            "foo.toto()[a]",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (_rest, parsed) = parse_primary(input).finish().unwrap();

        let secondaries = parsed.secondaries.unwrap();

        assert_eq!(secondaries.len(), 3);
    }
}

#[cfg(test)]
mod parse_fn_decl {
    use super::*;

    #[test]
    fn valid_no_args() {
        let input = Parser::new_extra(
            "toto =\n  2\n",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (_rest, parsed) = parse_fn(input).finish().unwrap();

        let expected = FunctionDecl {
            name: Identifier {
                name: String::from("toto"),
                node_id: 0,
            },
            body: Body::new(vec![Statement::Expression(Box::new(
                Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
                    op: Operand::Literal(Literal {
                        kind: LiteralKind::Number(2),
                        node_id: 0,
                    }),
                    node_id: 0,
                    secondaries: None,
                })),
            ))]),
            arguments: vec![],
            signature: FuncType {
                ret: Box::new(Type::forall("a")),
                arguments: vec![],
            },
            node_id: 0,
        };

        assert_eq!(parsed.name, expected.name);
        assert_eq!(parsed.arguments, expected.arguments);
        assert_eq!(parsed.signature, expected.signature);
    }

    #[test]
    fn valid_2_args() {
        let operators = HashMap::from([("+".to_string(), 5)]);

        let input = Parser::new_extra(
            "toto a b =\n  a + b",
            ParserCtx::new_with_operators(PathBuf::new(), operators, Config::default()),
        );

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
            node_id: 0,
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

    #[test]
    fn valid_multiline() {
        let operators = HashMap::from([("+".to_string(), 5)]);

        let input = Parser::new_extra(
            "toto a b =\n  a + b\n  a + b",
            ParserCtx::new_with_operators(PathBuf::new(), operators, Config::default()),
        );

        let (rest, _parsed) = parse_fn(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }

    #[test]
    fn valid_one_line() {
        let operators = HashMap::from([("+".to_string(), 5)]);

        let input = Parser::new_extra(
            "toto a b = a + b",
            ParserCtx::new_with_operators(PathBuf::new(), operators, Config::default()),
        );

        let (rest, _parsed) = parse_fn(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }
}

#[cfg(test)]
mod parse_prototype {
    use super::*;

    #[test]
    fn valid() {
        let input = Parser::new_extra(
            "toto :: Int64 -> Int64",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (_rest, parsed) = parse_prototype(input).finish().unwrap();

        let expected = Prototype {
            name: Identifier {
                name: String::from("toto"),
                node_id: 0,
            },
            signature: FuncType {
                ret: Box::new(Type::int64()),
                arguments: vec![Type::int64()],
            },
            node_id: 0,
        };

        assert_eq!(parsed.name, expected.name);
    }
}

#[cfg(test)]
mod parse_use {
    use super::*;

    #[test]
    fn valid() {
        let input = Parser::new_extra("use foo", ParserCtx::new(PathBuf::new(), Config::default()));

        let (_rest, parsed) = parse_use(input).finish().unwrap();

        assert_eq!(
            parsed.path,
            IdentifierPath {
                path: vec![Identifier {
                    name: String::from("foo"),
                    node_id: 0,
                }],
            }
        );
    }
}

#[cfg(test)]
mod parse_if {
    use super::*;

    #[test]
    fn valid_if() {
        let input = Parser::new_extra(
            "if a\nthen b",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_if(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }

    #[test]
    fn valid_if_else() {
        let input = Parser::new_extra(
            "if a\nthen b\nelse c",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_if(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }

    #[test]
    fn valid_if_else_if() {
        let input = Parser::new_extra(
            "if a\nthen b\nelse if false\nthen c",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_if(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }

    #[test]
    fn valid_if_else_if_else() {
        let input = Parser::new_extra(
            "if a\nthen b\nelse if true\nthen c\nelse d",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_if(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }

    #[test]
    fn valid_multiline_if_else() {
        let input = Parser::new_extra(
            "if a\nthen\n  b\nelse\n  d",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_if(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }
}

#[cfg(test)]
mod parse_for {
    use super::*;

    #[test]
    fn valid_for_in() {
        let input = Parser::new_extra(
            "for x in a\n  b",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_for(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }

    #[test]
    fn valid_while() {
        let input = Parser::new_extra(
            "while a\n  b = 2",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_for(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }
}

#[cfg(test)]
mod parse_assign {
    use super::*;

    #[test]
    fn valid_assign() {
        let input = Parser::new_extra(
            "let a = 2",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_assign(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }

    #[test]
    fn valid_reassign_ident() {
        let input = Parser::new_extra("a = 2", ParserCtx::new(PathBuf::new(), Config::default()));

        let (rest, _parsed) = parse_assign(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }

    #[test]
    fn valid_reassign_dot() {
        let input = Parser::new_extra("a.b = 2", ParserCtx::new(PathBuf::new(), Config::default()));

        let (rest, _parsed) = parse_assign(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }

    #[test]
    fn valid_reassign_indice() {
        let input = Parser::new_extra(
            "a[2] = 2",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_assign(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }
}

#[cfg(test)]
mod parse_struct_decl {
    use super::*;

    #[test]
    fn valid_struct_decl() {
        let input = Parser::new_extra(
            "struct Foo",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_struct_decl(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }

    #[test]
    fn valid_struct_decl_with_fields() {
        let input = Parser::new_extra(
            "struct Foo\n  a :: Int64\n  b :: Float64",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_struct_decl(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }
}

#[cfg(test)]
mod parse_struct_ctor {
    use super::*;

    #[test]
    fn valid_struct_ctor() {
        let input = Parser::new_extra("Foo\n", ParserCtx::new(PathBuf::new(), Config::default()));

        let (rest, _parsed) = parse_struct_ctor(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }

    #[test]
    fn valid_struct_ctor_with_fields() {
        let input = Parser::new_extra(
            "Foo\n  a: 2\n  b: 3.0",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_struct_ctor(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }
}

#[cfg(test)]
mod parse_trait {
    use super::*;

    #[test]
    fn valid_trait() {
        let input = Parser::new_extra(
            "trait Foo\n  a :: Int64 -> Int64\n  b :: Float64 -> String",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_trait(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }
}

#[cfg(test)]
mod parse_impl {
    use super::*;

    #[test]
    fn valid_impl() {
        let input = Parser::new_extra(
            "impl Foo\n  a =\n    2\n  b a =\n    a",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_impl(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }
}

#[cfg(test)]
mod parse_native_operator {
    use super::*;

    #[test]
    fn valid_native_operator() {
        let input = Parser::new_extra(
            "~IAdd a b",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (_rest, (parsed, _, _)) = parse_native_operator(input).finish().unwrap();

        assert_eq!(parsed.kind, NativeOperatorKind::IAdd);
    }
}

#[cfg(test)]
mod parse_array {
    use super::*;

    #[test]
    fn valid_array() {
        let input = Parser::new_extra(
            "[1, 2, 3]",
            ParserCtx::new(PathBuf::new(), Config::default()),
        );

        let (rest, _parsed) = parse_array(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }
}

#[cfg(test)]
mod parse_string {
    use super::*;

    #[test]
    fn valid_string() {
        let input = Parser::new_extra("\"foo\"", ParserCtx::new(PathBuf::new(), Config::default()));

        let (rest, _parsed) = parse_string(input).finish().unwrap();

        assert!(rest.fragment().is_empty());
    }
}
