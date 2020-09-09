use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Class;
use crate::ast::FunctionDecl;
use crate::ast::Parse;
use crate::ast::Prototype;
use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;
use llvm_sys::LLVMValue;

use crate::generator::Generate;
use crate::try_or_restore;

#[derive(Debug, Clone)]
pub enum TopLevel {
    Mod(String),
    Class(Class),
    Function(FunctionDecl),
    Prototype(Prototype),
}

impl Parse for TopLevel {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let res = match ctx.cur_tok.t {
            TokenType::ExternKeyword => {
                ctx.save();

                ctx.consume();

                let proto = try_or_restore!(Prototype::parse(ctx), ctx);

                ctx.save_pop();

                Ok(TopLevel::Prototype(proto))
            }
            TokenType::ClassKeyword => {
                ctx.save();

                ctx.consume();

                let class = try_or_restore!(Class::parse(ctx), ctx);

                ctx.save_pop();

                Ok(TopLevel::Class(class))
            }
            _ => Ok(TopLevel::Function(FunctionDecl::parse(ctx)?)),
        };

        while ctx.cur_tok.t == TokenType::EOL {
            ctx.consume();
        }

        res
    }
}

impl TypeInferer for TopLevel {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("TopLevel");

        match self {
            TopLevel::Class(class) => class.infer(ctx),
            TopLevel::Function(fun) => fun.infer(ctx),
            TopLevel::Prototype(fun) => fun.infer(ctx),
            TopLevel::Mod(_) => Err(Error::new_empty()),
        }
    }
}

impl Generate for TopLevel {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        match self {
            TopLevel::Class(class) => class.generate(ctx),
            TopLevel::Function(fun) => fun.generate(ctx),
            TopLevel::Prototype(fun) => fun.generate(ctx),
            TopLevel::Mod(_) => Err(Error::new_empty()),
        }
    }
}

impl IrBuilder for TopLevel {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        match self {
            TopLevel::Class(class) => class.build(context),
            TopLevel::Function(fun) => fun.build(context),
            TopLevel::Prototype(fun) => fun.build(context),
            TopLevel::Mod(_) => None,
        };

        None
    }
}
