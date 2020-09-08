use crate::Error;
use crate::Parser;
use crate::Token;
use crate::TokenType;

use crate::ast::Parse;
use crate::ast::Type;

use crate::parser::macros::*;

pub type ArgumentsDecl = Vec<ArgumentDecl>;

impl Parse for ArgumentsDecl {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut res = vec![];

        ctx.save();

        let mut has_parens = false;

        if TokenType::OpenParens == ctx.cur_tok.t {
            ctx.consume();

            has_parens = true;
        }

        if has_parens && TokenType::CloseParens == ctx.cur_tok.t {
            ctx.consume();

            ctx.save_pop();

            return Ok(res);
        }

        loop {
            let arg = try_or_restore!(ArgumentDecl::parse(ctx), ctx);

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
pub struct ArgumentDecl {
    pub name: String,
    pub t: Option<Type>,
    pub token: Token,
}

impl Parse for ArgumentDecl {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = expect!(TokenType::Identifier(ctx.cur_tok.txt.clone()), ctx);

        let name = token.txt.clone();

        ctx.save();

        let t = if ctx.cur_tok.t == TokenType::SemiColon {
            expect_or_restore!(TokenType::SemiColon, ctx);

            Some(try_or_restore_expect!(
                Type::parse(ctx),
                TokenType::Type(ctx.cur_tok.txt.clone()),
                ctx
            ))
        } else {
            None
        };

        ctx.save_pop();

        Ok(ArgumentDecl { name, t, token })
    }
}
