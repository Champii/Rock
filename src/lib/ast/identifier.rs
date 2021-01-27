use crate::Error;
use crate::Parser;
use crate::{ast::helper::*, token::TokenType};

use crate::ast::Parse;

use crate::parser::macros::*;

use super::Identity;

#[derive(Debug, Clone)]
pub struct Identifier {
    pub name: String,
    pub identity: Identity,
}

impl std::ops::Deref for Identifier {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

generate_has_name!(Identifier);

impl Parse for Identifier {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;

        let token = expect!(TokenType::Identifier(ctx.cur_tok().txt.clone()), ctx);

        Ok(Self {
            name: token.txt,
            identity: Identity::new(token_id),
        })
    }
}

// impl Annotate for Identifier {
//     fn annotate(&self, ctx: &mut InferBuilder) {
//         ctx.new_named_annotation(self.name.clone(), self.identity.clone());
//     }
// }

// impl ConstraintGen for Identifier {
//     fn constrain(&self, ctx: &mut InferBuilder) -> TypeId {
//         ctx.get_type_id(self.identity.clone()).unwrap()
//     }
// }
