use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::OperandKind;
use crate::ast::Operator;
use crate::ast::Parse;
use crate::ast::PrimaryExpr;
use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use llvm_sys::LLVMValue;

use crate::try_or_restore;

#[derive(Debug, Clone)]
pub enum UnaryExpr {
    PrimaryExpr(PrimaryExpr),
    UnaryExpr(Operator, Box<UnaryExpr>),
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
                            Some(i.clone())
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
        if ctx.cur_tok.t == TokenType::Operator(ctx.cur_tok.txt.clone()) {
            ctx.save();

            let op = try_or_restore!(Operator::parse(ctx), ctx);

            let unary = try_or_restore!(UnaryExpr::parse(ctx), ctx);

            ctx.save_pop();

            return Ok(UnaryExpr::UnaryExpr(op, Box::new(unary)));
        }

        Ok(UnaryExpr::PrimaryExpr(PrimaryExpr::parse(ctx)?))
    }
}

impl TypeInferer for UnaryExpr {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("UnaryExpr");

        match self {
            UnaryExpr::PrimaryExpr(primary) => primary.infer(ctx),
            UnaryExpr::UnaryExpr(_op, unary) => unary.infer(ctx),
        }
    }
}

impl IrBuilder for UnaryExpr {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        match self {
            UnaryExpr::PrimaryExpr(primary) => primary.build(context),
            UnaryExpr::UnaryExpr(_op, unary) => unary.build(context),
        }
    }
}
