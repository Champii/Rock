use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Identifier;
use crate::ast::Parse;
use crate::ast::Type;

use crate::parser::macros::*;

// TODO: Check if Default is mandatory here
#[derive(Debug, Clone)]
pub struct Selector {
    pub name: Identifier,
    pub class_offset: u8,
    pub class_type: Option<Type>,
    pub full_name: Identifier, // after generation and type infer
}

impl Parse for Selector {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        ctx.save();

        expect_or_restore!(TokenType::Dot, ctx);

        let expr = try_or_restore!(Identifier::parse(ctx), ctx);

        ctx.save_pop();

        let sel = Selector {
            name: expr.clone(),
            class_offset: 0,
            class_type: None,
            full_name: expr,
        };

        return Ok(sel);
    }
}
