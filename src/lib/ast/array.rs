use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Expression;
use crate::ast::Parse;
use crate::ast::Type;

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct Array {
    pub items: Vec<Expression>,
    pub t: Option<Type>,
}

impl Parse for Array {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        ctx.save();

        expect_or_restore!(TokenType::OpenArray, ctx);

        let mut items = vec![];

        while ctx.cur_tok.t != TokenType::CloseArray {
            let item = try_or_restore!(Expression::parse(ctx), ctx);

            items.push(item);

            if ctx.cur_tok.t != TokenType::Coma && ctx.cur_tok.t != TokenType::CloseArray {
                ctx.restore();
            }

            if ctx.cur_tok.t == TokenType::Coma {
                ctx.consume();
            }
        }

        expect_or_restore!(TokenType::CloseArray, ctx);

        ctx.save_pop();

        Ok(Array { items, t: None })
    }
}
