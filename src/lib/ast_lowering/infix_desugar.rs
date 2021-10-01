use std::collections::HashMap;

use crate::ast::{Expression, ExpressionKind, Identifier, Operand, UnaryExpr};

#[derive(Debug)]
pub enum ExprOrIdentifier {
    Expr(UnaryExpr),
    Identifier(Identifier),
}

#[derive(Debug)]
pub struct InfixDesugar {
    pub opstack: Vec<Identifier>,
    pub output: Vec<ExprOrIdentifier>,
    pub operators_list: HashMap<String, u8>,
}

impl InfixDesugar {
    pub fn new(operators_list: HashMap<String, u8>) -> Self {
        InfixDesugar {
            opstack: vec![],
            output: vec![],
            operators_list,
        }
    }

    pub fn desugar(&mut self, expr: &Expression) -> Expression {
        self.populate_rec(expr);

        self.pop_higher_operators(0);

        self.generate_calls()
    }

    pub fn generate_calls(&self) -> Expression {
        let mut stack = vec![];
        for item in &self.output {
            match item {
                ExprOrIdentifier::Expr(expr) => stack.push(expr.clone()),
                ExprOrIdentifier::Identifier(id) => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();

                    stack.push(UnaryExpr::create_2_args_func_call(
                        Operand::from_identifier(id),
                        left.clone(),
                        right.clone(),
                    ));
                }
            }
        }

        Expression::from_unary(&stack.pop().unwrap())
    }

    pub fn populate_rec(&mut self, expr: &Expression) {
        match &expr.kind {
            ExpressionKind::BinopExpr(unary, op, expr2) => {
                self.output.push(ExprOrIdentifier::Expr(unary.clone()));

                self.pop_higher_operators(
                    *self.operators_list.get(&op.0.name.to_string()).unwrap(),
                );

                self.opstack.push(op.0.clone());

                self.desugar(expr2);
            }
            ExpressionKind::UnaryExpr(unary) => {
                self.output.push(ExprOrIdentifier::Expr(unary.clone()))
            }
            _ => unimplemented!(),
        }
    }

    pub fn pop_higher_operators(&mut self, precedence: u8) {
        for op in self.opstack.clone().iter().rev() {
            let item_precedence = self.operators_list.get(&op.name.to_string()).unwrap();

            if precedence <= *item_precedence {
                self.opstack.pop();

                self.output.push(ExprOrIdentifier::Identifier(op.clone()));
            } else {
                break;
            }
        }
    }
}
