use crate::Parser;
use crate::Token;
use crate::TokenType;
use crate::{token::TokenId, Error};

use crate::ast::ast_print::*;
use crate::ast::Expression;
use crate::ast::Parse;
// use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
// use crate::type_checker::TypeInferer;

use llvm_sys::LLVMValue;

use crate::generator::Generate;
use crate::parser::macros::*;

pub type Arguments = Vec<Argument>;

impl Parse for Arguments {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut res = vec![];

        ctx.save();

        // let has_parens = if TokenType::OpenParens == ctx.cur_tok.t {
        //     expect!(TokenType::OpenParens, ctx);

        //     true
        // } else {
        //     false
        // };

        // if has_parens && TokenType::CloseParens == ctx.cur_tok.t {
        //     ctx.consume();

        //     ctx.save_pop();

        //     return Ok(res);
        // }

        loop {
            let arg = try_or_restore!(Argument::parse(ctx), ctx);

            res.push(arg);

            if TokenType::EOL == ctx.cur_tok().t {
                break;
            }

            ctx.consume();
        }

        ctx.save_pop();

        Ok(res)
    }
}

#[derive(Debug, Clone)]
pub struct Argument {
    pub arg: Expression,
    // pub t: TypeInfer,
    pub token: TokenId,
}

derive_print!(Argument, [arg]);

impl Parse for Argument {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok_id;

        Ok(Argument {
            arg: Expression::parse(ctx)?,
            // t: None,
            token,
        })
    }
}

// impl TypeInferer for Argument {
//     fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
//         trace!("Argument ({:?})", self.token);

//         let t = self.arg.infer(ctx);

//         self.t = t?;

//         Ok(self.t.clone())
//     }
// }

// impl Generate for Argument {
//     fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
//         self.arg.generate(ctx)
//     }
// }

// impl IrBuilder for Argument {
//     fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
//         self.arg.build(context)
//     }
// }
