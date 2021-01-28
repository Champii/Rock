use crate::{
    ast::{identity::Identity, Mod},
    parser::Parser,
    Error,
};

use super::Parse;

impl Parse for Root {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        Ok(Root {
            identity: Identity::new(ctx.cur_tok_id),
            r#mod: Mod::parse(ctx)?,
        })
    }
}
