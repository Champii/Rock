use crate::Parser;
use crate::Token;
use crate::{token::TokenId, Error};

use crate::ast::Operator;
use crate::ast::Parse;
// use crate::ast::PrimitiveType;
// use crate::ast::Type;
// use crate::ast::TypeInfer;
use crate::ast::ast_print::*;
use crate::ast::UnaryExpr;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
// use crate::type_checker::TypeInferer;

use llvm::LLVMIntPredicate;
use llvm_sys::core::LLVMBuildAdd;
use llvm_sys::core::LLVMBuildICmp;
use llvm_sys::LLVMValue;

use crate::generator::Generate;
use crate::try_or_restore_and;

#[derive(Debug, Clone)]
pub enum ExpressionKind {
    BinopExpr(UnaryExpr, Operator, Box<Expression>),
    UnaryExpr(UnaryExpr),
}

impl AstPrint for ExpressionKind {
    fn print(&self, ctx: &mut AstPrintContext) {
        match self {
            Self::UnaryExpr(u) => u.print(ctx),
            Self::BinopExpr(b, op, e) => {
                b.print(ctx);
                op.print(ctx);
                e.print(ctx);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub token: TokenId,
}

derive_print!(Expression, [kind]);

impl Expression {
    pub fn is_literal(&self) -> bool {
        match &self.kind {
            ExpressionKind::UnaryExpr(unary) => unary.is_literal(),
            _ => false,
        }
    }

    pub fn is_identifier(&self) -> bool {
        match &self.kind {
            ExpressionKind::UnaryExpr(unary) => unary.is_identifier(),
            _ => false,
        }
    }

    pub fn get_identifier(&self) -> Option<String> {
        match &self.kind {
            ExpressionKind::UnaryExpr(unary) => unary.get_identifier(),
            _ => None,
        }
    }
}

impl Parse for Expression {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok_id;

        let left = UnaryExpr::parse(ctx)?;

        let mut res = Expression {
            kind: ExpressionKind::UnaryExpr(left.clone()),
            token,
        };

        ctx.save();

        let op = try_or_restore_and!(Operator::parse(ctx), Ok(res), ctx);

        let right = try_or_restore_and!(Expression::parse(ctx), Ok(res), ctx);

        ctx.save_pop();

        res.kind = ExpressionKind::BinopExpr(left, op, Box::new(right));

        Ok(res)
    }
}

// impl TypeInferer for Expression {
//     fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
//         trace!("Expression ({:?})", self.token);

//         match &mut self.kind {
//             ExpressionKind::BinopExpr(unary, op, expr) => {
//                 let t = match op {
//                     Operator::Add => Some(Type::Primitive(PrimitiveType::Int32)),
//                     Operator::EqualEqual => Some(Type::Primitive(PrimitiveType::Bool)),
//                     _ => Some(Type::Primitive(PrimitiveType::Int32)),
//                 };

//                 ctx.cur_type = t.clone();

//                 let left = unary.infer(ctx)?;
//                 let right = expr.infer(ctx)?;

//                 if left != right {
//                     return Err(Error::new_type_error(
//                         ctx.input.clone(),
//                         self.token.clone(),
//                         left,
//                         right,
//                     ));
//                 }

//                 // ctx.cur_type = None;

//                 self.t = t.clone();

//                 Ok(t)
//             }
//             ExpressionKind::UnaryExpr(unary) => {
//                 let t = unary.infer(ctx)?;

//                 self.t = t.clone();

//                 Ok(t)
//             }
//         }
//     }
// }

// impl Generate for Expression {
//     fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
//         match &mut self.kind {
//             ExpressionKind::BinopExpr(unary, _op, expr) => {
//                 let _left = unary.generate(ctx)?;
//                 let _right = expr.generate(ctx)?;

//                 Ok(())
//             }
//             ExpressionKind::UnaryExpr(unary) => unary.generate(ctx),
//         }
//     }
// }

// impl IrBuilder for Expression {
//     fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
//         match &self.kind {
//             ExpressionKind::BinopExpr(unary, op, expr) => {
//                 let left = unary.build(context).unwrap();
//                 let right = expr.build(context).unwrap();

//                 Some(match op {
//                     Operator::Add => unsafe {
//                         LLVMBuildAdd(context.builder, left, right, b"\0".as_ptr() as *const _)
//                     },
//                     Operator::EqualEqual => unsafe {
//                         LLVMBuildICmp(
//                             context.builder,
//                             LLVMIntPredicate::LLVMIntEQ,
//                             left,
//                             right,
//                             "\0".as_ptr() as *const _,
//                         )
//                     },
//                     Operator::DashEqual => unsafe {
//                         LLVMBuildICmp(
//                             context.builder,
//                             LLVMIntPredicate::LLVMIntNE,
//                             left,
//                             right,
//                             "\0".as_ptr() as *const _,
//                         )
//                     },
//                     Operator::Less => unsafe {
//                         LLVMBuildICmp(
//                             context.builder,
//                             LLVMIntPredicate::LLVMIntSLT,
//                             left,
//                             right,
//                             "\0".as_ptr() as *const _,
//                         )
//                     },
//                     Operator::LessOrEqual => unsafe {
//                         LLVMBuildICmp(
//                             context.builder,
//                             LLVMIntPredicate::LLVMIntSLE,
//                             left,
//                             right,
//                             "\0".as_ptr() as *const _,
//                         )
//                     },
//                     Operator::More => unsafe {
//                         LLVMBuildICmp(
//                             context.builder,
//                             LLVMIntPredicate::LLVMIntSGT,
//                             left,
//                             right,
//                             "\0".as_ptr() as *const _,
//                         )
//                     },
//                     Operator::MoreOrEqual => unsafe {
//                         LLVMBuildICmp(
//                             context.builder,
//                             LLVMIntPredicate::LLVMIntSGE,
//                             left,
//                             right,
//                             "\0".as_ptr() as *const _,
//                         )
//                     },
//                     _ => unsafe {
//                         LLVMBuildAdd(context.builder, left, right, b"\0".as_ptr() as *const _)
//                     },
//                 })
//             }
//             ExpressionKind::UnaryExpr(unary) => unary.build(context),
//         }
//     }
// }
