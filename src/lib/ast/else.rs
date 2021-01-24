use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Body;
use crate::ast::If;
use crate::ast::Parse;

#[derive(Debug, Clone)]
pub enum Else {
    If(If),
    Body(Body),
}

impl Parse for Else {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        Ok(match ctx.cur_tok().t {
            TokenType::IfKeyword => Else::If(If::parse(ctx)?),
            _ => Else::Body(Body::parse(ctx)?),
        })
    }
}
