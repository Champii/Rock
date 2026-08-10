use crate::ast::{Expression, LiteralKind, Operand, UnaryExpr};
use crate::parser::items::tests::common::parse_literal;

#[test]
fn test_parse_array_repeat() {
    let literal = parse_literal("[1 + 2;4]");

    let LiteralKind::ArrayRepeat { value, len } = literal.kind else {
        panic!("expected repeat array literal");
    };

    assert_eq!(len, 4);
    assert!(matches!(*value, Expression::BinopExpr(_, _, _)));
}

#[test]
fn test_parse_nested_array_repeat() {
    let literal = parse_literal("[[0; 2]; 3]");

    let LiteralKind::ArrayRepeat { value, len: 3 } = literal.kind else {
        panic!("expected outer repeat array literal");
    };
    let Expression::UnaryExpr(UnaryExpr::PrimaryExpr(primary)) = *value else {
        panic!("expected inner repeat array expression");
    };
    assert!(matches!(
        primary.operand,
        Operand::Literal(crate::ast::Literal {
            kind: LiteralKind::ArrayRepeat { len: 2, .. },
            ..
        })
    ));
}
