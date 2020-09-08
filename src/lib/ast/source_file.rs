use crate::ast::Parse;
use crate::ast::TopLevel;
use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub top_levels: Vec<TopLevel>,
}

impl Parse for SourceFile {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut top_levels = vec![];

        while ctx.cur_tok.t != TokenType::EOF {
            top_levels.push(TopLevel::parse(ctx)?);
        }

        expect!(TokenType::EOF, ctx);

        Ok(SourceFile { top_levels })
    }
}
