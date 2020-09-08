use crate::Error;
use crate::Parser;
use crate::Token;
use crate::TokenType;

use crate::ast::Expression;
use crate::ast::Parse;
use crate::ast::TypeInfer;

use crate::parser::macros::*;

pub type Arguments = Vec<Argument>;

impl Parse for Arguments {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut res = vec![];

        ctx.save();

        let has_parens = if TokenType::OpenParens == ctx.cur_tok.t {
            expect!(TokenType::OpenParens, ctx);

            true
        } else {
            false
        };

        if has_parens && TokenType::CloseParens == ctx.cur_tok.t {
            ctx.consume();

            ctx.save_pop();

            return Ok(res);
        }

        loop {
            let arg = try_or_restore!(Argument::parse(ctx), ctx);

            res.push(arg);

            if TokenType::Coma != ctx.cur_tok.t {
                break;
            }

            ctx.consume();
        }

        if has_parens {
            expect_or_restore!(TokenType::CloseParens, ctx);
        }

        ctx.save_pop();

        Ok(res)
    }
}

#[derive(Debug, Clone)]
pub struct Argument {
    pub arg: Expression,
    pub t: TypeInfer,
    pub token: Token,
}

impl Parse for Argument {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok.clone();

        Ok(Argument {
            arg: Expression::parse(ctx)?,
            t: None,
            token,
        })
    }
}
