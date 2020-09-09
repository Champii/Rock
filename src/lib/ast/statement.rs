use crate::Error;
use crate::Parser;
use crate::Token;

use crate::ast::Assignation;
use crate::ast::Expression;
use crate::ast::For;
use crate::ast::If;
use crate::ast::Parse;
use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use llvm_sys::LLVMValue;

use crate::error;

#[derive(Debug, Clone)]
pub enum StatementKind {
    If(If),
    For(For),
    Expression(Expression),
    Assignation(Assignation),
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: StatementKind,
    pub t: TypeInfer,
    pub token: Token,
}

impl Parse for Statement {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok.clone();

        let kind = if let Ok(if_) = If::parse(ctx) {
            StatementKind::If(if_)
        } else if let Ok(for_) = For::parse(ctx) {
            StatementKind::For(for_)
        } else if let Ok(assign) = Assignation::parse(ctx) {
            StatementKind::Assignation(assign)
        } else if let Ok(expr) = Expression::parse(ctx) {
            StatementKind::Expression(expr)
        } else {
            error!("Expected statement".to_string(), ctx);
        };

        Ok(Statement {
            kind,
            t: None,
            token,
        })
    }
}

impl TypeInferer for Statement {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("Statement ({:?})", self.token);

        let t = match &mut self.kind {
            StatementKind::If(if_) => if_.infer(ctx),
            StatementKind::For(for_) => for_.infer(ctx),
            StatementKind::Expression(expr) => expr.infer(ctx),
            StatementKind::Assignation(assign) => assign.infer(ctx),
        };

        self.t = t?;

        Ok(self.t.clone())
    }
}

impl IrBuilder for Statement {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        match &self.kind {
            StatementKind::If(if_) => if_.build(context),
            StatementKind::For(for_) => for_.build(context),
            StatementKind::Expression(expr) => expr.build(context),
            StatementKind::Assignation(assign) => assign.build(context),
        }
    }
}
