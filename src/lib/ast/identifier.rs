use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;

use crate::parser::macros::*;

pub type Identifier = String;

impl Parse for Identifier {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        Ok(expect!(TokenType::Identifier(ctx.cur_tok().txt.clone()), ctx).txt)
    }
}
