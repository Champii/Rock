use crate::ast::*;
use crate::parser::items::*;
use crate::parser::*;
use crate::Config;

#[test]
fn application_binds_before_infix() {
    let input = "foo 2 + 2";
    let tokens = lex_test(input);
    let config = Config::default();

    let (rest, expression) = expression
        .process(ParseCtx::from(&tokens, &config))
        .unwrap();

    match expression {
        Expression::BinopExpr(lhs, op, rhs) => {
            assert_eq!(op.value, "+");

            match lhs {
                UnaryExpr::PrimaryExpr(PrimaryExpr {
                    operand: Operand::Ident(_),
                    secondaries: Some(secondaries),
                    ..
                }) => {
                    assert_eq!(secondaries.len(), 1);
                    assert!(matches!(&secondaries[0], SecondaryExpr::Arguments(_)));
                }
                other => panic!("expected application on the left, got {:?}", other),
            }

            match *rhs {
                Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
                    operand: Operand::Literal(_),
                    secondaries: None,
                    ..
                })) => {}
                other => panic!("expected literal rhs, got {:?}", other),
            }
        }
        other => panic!("expected binop, got {:?}", other),
    }

    assert_eq!(rest.len(), 0);
}

#[test]
fn constructor_application_stops_before_infix_chain() {
    let input = "Option::Some 2 <&> inc";
    let tokens = lex_test(input);
    let config = Config::default();

    let (rest, expression) = expression
        .process(ParseCtx::from(&tokens, &config))
        .unwrap();

    assert!(matches!(expression, Expression::BinopExpr(_, _, _)));
    assert_eq!(rest.len(), 0);
}
