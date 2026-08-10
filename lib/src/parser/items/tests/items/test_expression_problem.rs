use crate::parser::*;
use crate::Config;

#[test]
fn test_simple_function() {
    let input = "main = ->\n    1\n";
    let config = Config::default();

    let result = parse_string(input, &config);
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_function_with_method_call() {
    let input = "main = ->\n    foo\n        .bar\n";
    let config = Config::default();

    let result = parse_string(input, &config);
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_function_with_method_and_arg() {
    let input = "main = ->\n    foo\n        .bar 1\n";
    let config = Config::default();

    let result = parse_string(input, &config);
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_function_with_lambda_arg() {
    let input = "main = ->\n    foo\n        .bar ->\n            1\n";
    let config = Config::default();

    let result = parse_string(input, &config);
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_function_with_param_lambda_arg() {
    let input = "main = ->\n    foo\n        .bar x ->\n            1\n";
    let config = Config::default();

    let result = parse_string(input, &config);
    assert!(result.is_ok(), "Failed: {:?}", result.err());
}

#[test]
fn test_expression_problem_full() {
    let input = "main = ->\n    foo\n        .bar lol ->\n            mdr\n        .haha\n";
    let config = Config::default();

    let result = parse_string(input, &config);
    assert!(result.is_ok(), "Failed: {:?}", result.err());

    // Verify the AST structure: the .haha should be outside the lambda body
    let ast = result.unwrap();
    match &ast.module.top_levels[0] {
        crate::ast::TopLevel::FunctionDecl(func) => {
            match &func.lambda.body.statements[0] {
                crate::ast::Statement::Expression(expr) => {
                    match expr {
                        crate::ast::Expression::UnaryExpr(crate::ast::UnaryExpr::PrimaryExpr(
                            primary,
                        )) => {
                            // Should have 3 secondaries: .bar, arguments, .haha
                            assert_eq!(
                                primary.secondaries.as_ref().unwrap().len(),
                                3,
                                "Expected 3 secondaries: .bar, arguments, .haha"
                            );

                            // The third secondary should be .haha
                            match &primary.secondaries.as_ref().unwrap()[2] {
                                crate::ast::SecondaryExpr::Dot(
                                    crate::ast::IdentOrNumber::Ident(ident),
                                ) => {
                                    assert_eq!(
                                        ident.name, "haha",
                                        ".haha should be outside the lambda body"
                                    );
                                }
                                _ => panic!("Expected .haha as third secondary"),
                            }
                        }
                        _ => panic!("Expected PrimaryExpr"),
                    }
                }
                _ => panic!("Expected Expression statement"),
            }
        }
        _ => panic!("Expected FunctionDecl"),
    }
}
