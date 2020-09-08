use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::ForIn;
use crate::ast::Parse;
use crate::ast::While;

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub enum For {
    In(ForIn),
    While(While),
}

impl Parse for For {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        expect!(TokenType::ForKeyword, ctx);

        ctx.save();

        let res = if let Ok(forin) = ForIn::parse(ctx) {
            For::In(forin)
        } else if let Ok(while_) = While::parse(ctx) {
            For::While(while_)
        } else {
            ctx.restore();

            self::error!("Bad for".to_string(), ctx);
        };

        ctx.save_pop();

        Ok(res)
    }
}
