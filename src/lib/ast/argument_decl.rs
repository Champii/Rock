use super::Identity;
use crate::parser::macros::*;
use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;

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

#[derive(Debug, Clone, Default)]
pub struct ArgumentDecl {
    pub name: String,
    pub identity: Identity,
}

impl Parse for ArgumentDecl {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;

        let token = expect!(TokenType::Identifier(ctx.cur_tok().txt.clone()), ctx);

        Ok(ArgumentDecl {
            name: token.txt.clone(),
            identity: Identity::new(token_id),
        })
    }
}

// impl Annotate for ArgumentDecl {
//     fn annotate(&self, ctx: &mut InferBuilder) {
//         ctx.new_named_annotation(self.name.clone(), self.identity.clone());
//     }
// }

// impl ConstraintGen for ArgumentDecl {
//     fn constrain(&self, ctx: &mut InferBuilder) -> TypeId {
//         ctx.get_type_id(self.identity.clone()).unwrap()
//     }
// }
