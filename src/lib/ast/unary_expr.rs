use crate::infer::*;
use crate::try_or_restore;
use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::OperandKind;
use crate::ast::Operator;
use crate::ast::Parse;
use crate::ast::PrimaryExpr;

#[derive(Debug, Clone)]
pub enum UnaryExpr {
    PrimaryExpr(PrimaryExpr),
    UnaryExpr(Operator, Box<UnaryExpr>),
}

impl ConstraintGen for UnaryExpr {
    fn constrain(&self, ctx: &mut InferBuilder) -> TypeId {
        // println!("Constraint: UnaryExpr");

        match self {
            UnaryExpr::PrimaryExpr(p) => p.constrain(ctx),
            UnaryExpr::UnaryExpr(op, unary) => {
                unary.constrain(ctx);

                op.constrain(ctx)
            }
        }
    }
}

impl UnaryExpr {
    pub fn is_literal(&self) -> bool {
        match self {
            UnaryExpr::PrimaryExpr(p) => match p {
                PrimaryExpr::PrimaryExpr(operand, _) => match &operand.kind {
                    OperandKind::Literal(_) => true,
                    _ => false,
                },
            },
            _ => false,
        }
    }

    pub fn is_identifier(&self) -> bool {
        match self {
            UnaryExpr::PrimaryExpr(p) => match p {
                PrimaryExpr::PrimaryExpr(operand, _) => match &operand.kind {
                    OperandKind::Identifier(_) => true,
                    _ => false,
                },
            },
            _ => false,
        }
    }

    pub fn get_identifier(&self) -> Option<String> {
        match self {
            UnaryExpr::PrimaryExpr(p) => match p {
                PrimaryExpr::PrimaryExpr(operand, vec) => match &operand.kind {
                    OperandKind::Identifier(i) => {
                        if vec.len() == 0 {
                            Some(i.name.clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
            },
            _ => None,
        }
    }
}

impl Parse for UnaryExpr {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if ctx.cur_tok().t == TokenType::Operator(ctx.cur_tok().txt.clone()) {
            ctx.save();

            let op = try_or_restore!(Operator::parse(ctx), ctx);

            let unary = try_or_restore!(UnaryExpr::parse(ctx), ctx);

            ctx.save_pop();

            return Ok(UnaryExpr::UnaryExpr(op, Box::new(unary)));
        }

        Ok(UnaryExpr::PrimaryExpr(PrimaryExpr::parse(ctx)?))
    }
}
