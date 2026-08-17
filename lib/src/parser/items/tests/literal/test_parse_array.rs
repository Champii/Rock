use crate::ast::*;
use crate::lexer::Span;
use crate::parser::items::tests::common::*;

#[test]
fn test_parse_array() {
    let input = "[1, 2, 3]";
    let literal = parse_literal(input);

    assert_eq!(
        literal.kind,
        LiteralKind::Array(Array {
            elements: vec![
                Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
                    operand: Operand::Literal(Literal {
                        kind: LiteralKind::Number(1),
                        span: Span::test(),
                    }),
                    secondaries: None,
                    type_annotation: None,
                })),
                Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
                    operand: Operand::Literal(Literal {
                        kind: LiteralKind::Number(2),
                        span: Span::test(),
                    }),
                    type_annotation: None,
                    secondaries: None,
                })),
                Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
                    operand: Operand::Literal(Literal {
                        kind: LiteralKind::Number(3),
                        span: Span::test(),
                    }),
                    secondaries: None,
                    type_annotation: None,
                })),
            ],
        })
    );
}
