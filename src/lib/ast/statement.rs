use crate::Parser;
use crate::Token;
use crate::{token::TokenId, Error};

// use crate::ast::Assignation;
use crate::ast::Expression;
// use crate::ast::For;
use crate::ast::If;
use crate::ast::Parse;
// use crate::ast::TypeInfer;
use crate::ast::ast_print::*;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
// use crate::type_checker::TypeInferer;

use crate::generator::Generate;
use llvm_sys::LLVMValue;

use crate::error;

#[derive(Debug, Clone)]
pub enum StatementKind {
    If(If),
    // For(For),
    Expression(Expression),
    // Assignation(Assignation),
}

impl AstPrint for StatementKind {
    fn print(&self, ctx: &mut AstPrintContext) {
        match self {
            // Self::If(f) => f.print(ctx),
            Self::Expression(e) => e.print(ctx),
            _ => (),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: Box<StatementKind>,
    pub token: TokenId,
}

derive_print!(Statement, [kind]);

impl Parse for Statement {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok_id;

        let kind = Box::new(if let Ok(if_) = If::parse(ctx) {
            StatementKind::If(if_)
        // } else if let Ok(for_) = For::parse(ctx) {
        //     StatementKind::For(for_)
        // } else if let Ok(assign) = Assignation::parse(ctx) {
        //     StatementKind::Assignation(assign)
        } else if let Ok(expr) = Expression::parse(ctx) {
            StatementKind::Expression(expr)
        } else {
            error!("Expected statement".to_string(), ctx);
        });

        Ok(Statement { kind, token })
    }
}

// impl TypeInferer for Statement {
//     fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
//         trace!("Statement ({:?})", self.token);

//         let t = match &mut self.kind {
//             StatementKind::If(if_) => if_.infer(ctx),
//             StatementKind::For(for_) => for_.infer(ctx),
//             StatementKind::Expression(expr) => expr.infer(ctx),
//             StatementKind::Assignation(assign) => assign.infer(ctx),
//         };

//         self.t = t?;

//         Ok(self.t.clone())
//     }
// }

// impl Generate for Statement {
//     fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
//         match &mut self.kind {
//             StatementKind::If(if_) => if_.generate(ctx),
//             StatementKind::For(for_) => for_.generate(ctx),
//             StatementKind::Expression(expr) => expr.generate(ctx),
//             StatementKind::Assignation(assign) => assign.generate(ctx),
//         }
//     }
// }

// impl IrBuilder for Statement {
//     fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
//         match &self.kind {
//             StatementKind::If(if_) => if_.build(context),
//             StatementKind::For(for_) => for_.build(context),
//             StatementKind::Expression(expr) => expr.build(context),
//             StatementKind::Assignation(assign) => assign.build(context),
//         }
//     }
// }
