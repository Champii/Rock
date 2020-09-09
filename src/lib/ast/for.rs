use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::ForIn;
use crate::ast::Parse;
use crate::ast::TypeInfer;
use crate::ast::While;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use llvm_sys::LLVMValue;

use crate::generator::Generate;
use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub enum For {
    In(ForIn),
    While(While),
}

impl Parse for For {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        expect!(TokenType::ForKeyword, ctx);

        ctx.save();

        let res = if let Ok(forin) = ForIn::parse(ctx) {
            For::In(forin)
        } else if let Ok(while_) = While::parse(ctx) {
            For::While(while_)
        } else {
            ctx.restore();

            self::error!("Bad for".to_string(), ctx);
        };

        ctx.save_pop();

        Ok(res)
    }
}

impl TypeInferer for For {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("For");

        match self {
            For::In(in_) => in_.infer(ctx),
            For::While(while_) => while_.infer(ctx),
        }
    }
}

impl Generate for For {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        match self {
            For::In(in_) => in_.generate(ctx),
            For::While(while_) => while_.generate(ctx),
        }
    }
}

impl IrBuilder for For {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        match self {
            For::In(in_) => in_.build(context),
            For::While(while_) => while_.build(context),
        }
    }
}
