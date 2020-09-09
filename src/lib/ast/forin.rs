use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Body;
use crate::ast::Expression;
use crate::ast::Identifier;
use crate::ast::Parse;
use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use llvm_sys::LLVMValue;

use crate::generator::Generate;
use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct ForIn {
    pub value: Identifier,
    pub expr: Expression,
    pub body: Body,
}

impl Parse for ForIn {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        ctx.save();

        let value = try_or_restore!(Identifier::parse(ctx), ctx);

        expect_or_restore!(TokenType::InKeyword, ctx);

        let expr = try_or_restore!(Expression::parse(ctx), ctx);

        let body = try_or_restore!(Body::parse(ctx), ctx);

        ctx.save_pop();

        Ok(ForIn { value, expr, body })
    }
}

impl TypeInferer for ForIn {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("ForIn");

        self.body.infer(ctx)
    }
}

impl Generate for ForIn {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        self.body.generate(ctx)
    }
}

impl IrBuilder for ForIn {
    fn build(&self, _context: &mut IrContext) -> Option<*mut LLVMValue> {
        panic!("ForIn: Uninplemented");
    }
}
