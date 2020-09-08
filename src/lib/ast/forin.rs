use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Body;
use crate::ast::Expression;
use crate::ast::Identifier;
use crate::ast::Parse;

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct ForIn {
    pub value: Identifier,
    pub expr: Expression,
    pub body: Body,
}

impl Parse for ForIn {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        ctx.save();

        let value = try_or_restore!(Identifier::parse(ctx), ctx);

        expect_or_restore!(TokenType::InKeyword, ctx);

        let expr = try_or_restore!(Expression::parse(ctx), ctx);

        let body = try_or_restore!(Body::parse(ctx), ctx);

        ctx.save_pop();

        Ok(ForIn { value, expr, body })
    }
}
