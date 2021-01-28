use crate::{
    ast::{identity::Identity, TopLevel},
    parser::{macros::*, Parse, Parser},
    Error, TokenType,
};

impl Parse for Mod {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut top_levels = vec![];

        while ctx.cur_tok().t != TokenType::EOF {
            top_levels.push(TopLevel::parse(ctx)?);
        }

        expect!(TokenType::EOF, ctx);

        Ok(Mod {
            top_levels,
            identity: Identity::new(0),
        })
    }
}
