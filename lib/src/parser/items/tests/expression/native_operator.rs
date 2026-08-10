use crate::ast::*;
use crate::lexer::Span;
use crate::parser::items::*;
use crate::parser::*;
use crate::Config;

#[test]
fn native_operator() {
    let input = "~IAdd";
    let tokens = lex_test(input);
    let config = Config::default();

    let (rest, expression) = expression
        .process(ParseCtx::from(&tokens, &config))
        .unwrap();

    assert_eq!(
        expression,
        Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
            operand: Operand::NativeOperator(NativeOperator {
                name: "IAdd".to_string(),
                span: Span::default(),
            }),
            secondaries: None,
            type_annotation: None,
        })),
    );
    assert_eq!(rest.len(), 0);
}
