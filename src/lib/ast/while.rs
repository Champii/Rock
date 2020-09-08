use crate::Error;
use crate::Parser;

use crate::ast::Body;
use crate::ast::Expression;
use crate::ast::Parse;

use crate::try_or_restore;

#[derive(Debug, Clone)]
pub struct While {
    pub predicat: Expression,
    pub body: Body,
}

impl Parse for While {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        ctx.save();

        let predicat = try_or_restore!(Expression::parse(ctx), ctx);

        let body = try_or_restore!(Body::parse(ctx), ctx);

        ctx.save_pop();

        Ok(While { predicat, body })
    }
}
