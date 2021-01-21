use crate::Parser;
use crate::Token;
use crate::TokenType;
use crate::{token::TokenId, Error};

use crate::ast::ast_print::*;
use crate::ast::Parse;
// use crate::ast::Type;
// use crate::ast::TypeInfer;

use crate::context::Context;
// use crate::type_checker::TypeInferer;

use crate::generator::Generate;
use crate::parser::macros::*;

pub type ArgumentsDecl = Vec<ArgumentDecl>;

impl Parse for ArgumentsDecl {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut res = vec![];

        ctx.save();

        loop {
            let arg = try_or_restore!(ArgumentDecl::parse(ctx), ctx);

            res.push(arg);

            match ctx.cur_tok().t {
                TokenType::Identifier(_) => {}
                _ => break,
            }
        }

        ctx.save_pop();

        Ok(res)
    }
}

#[derive(Debug, Clone)]
pub struct ArgumentDecl {
    pub name: String,
    pub token: TokenId,
}

derive_print!(ArgumentDecl, []);

impl Parse for ArgumentDecl {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;

        let token = expect!(TokenType::Identifier(ctx.cur_tok().txt.clone()), ctx);

        Ok(ArgumentDecl {
            name: token.txt.clone(),
            token: token_id,
        })
    }
}

// impl Generate for ArgumentDecl {
//     fn generate(&mut self, _ctx: &mut Context) -> Result<(), Error> {
//         Ok(())
//     }
// }

// impl TypeInferer for ArgumentDecl {
//     fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
//         trace!("ArgumentDecl ({:?})", self.token);

//         ctx.scopes.add(self.name.clone(), self.t.clone());

//         Ok(self.t.clone())
//     }
// }
