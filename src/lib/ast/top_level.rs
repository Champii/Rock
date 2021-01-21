use crate::infer::*;
use crate::TokenType;
use crate::{token::Token, Error};
use crate::{token::TokenId, Parser};

// use crate::ast::Class;
use crate::ast::ast_print::*;
use crate::ast::FunctionDecl;
use crate::ast::Parse;
// use crate::ast::Prototype;
// use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
// use crate::type_checker::TypeInferer;
use llvm_sys::LLVMValue;

use crate::generator::Generate;
use crate::try_or_restore;

#[derive(Debug, Clone)]
pub enum TopLevelKind {
    Function(FunctionDecl),
}

// visitable_enum!(TopLevelKind, Annotate, annotate, InferBuilder, [Function]);
// visitable_enum!(TopLevelKind, AstPrint, print, AstPrintContext, [Function]);

#[derive(Debug, Clone)]
pub struct TopLevel {
    pub kind: TopLevelKind,
    pub token: TokenId,
}

// derive_print!(TopLevel, [kind]);

impl Parse for TopLevel {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok_id;

        let kind = match ctx.cur_tok().t {
            _ => TopLevelKind::Function(FunctionDecl::parse(ctx)?),
        };

        while ctx.cur_tok().t == TokenType::EOL {
            ctx.consume();
        }

        Ok(TopLevel { kind, token })
    }
}

// impl TypeInferer for TopLevel {
//     fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
//         trace!("TopLevel");

//         match self {
//             TopLevel::Class(class) => class.infer(ctx),
//             TopLevel::Function(fun) => fun.infer(ctx),
//             TopLevel::Prototype(fun) => fun.infer(ctx),
//             TopLevel::Mod(_) => Err(Error::new_empty()),
//         }
//     }
// }

// impl Generate for TopLevel {
//     fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
//         match self {
//             TopLevel::Class(class) => class.generate(ctx),
//             TopLevel::Function(fun) => fun.generate(ctx),
//             TopLevel::Prototype(fun) => fun.generate(ctx),
//             TopLevel::Mod(_) => Err(Error::new_empty()),
//         }
//     }
// }

// impl IrBuilder for TopLevel {
//     fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
//         match self {
//             TopLevel::Class(class) => class.build(context),
//             TopLevel::Function(fun) => fun.build(context),
//             TopLevel::Prototype(fun) => fun.build(context),
//             TopLevel::Mod(_) => None,
//         };

//         None
//     }
// }
