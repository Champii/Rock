use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Body;
use crate::ast::Else;
use crate::ast::Expression;
use crate::ast::Parse;

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct If {
    pub predicat: Expression,
    pub body: Body,
    pub else_: Option<Box<Else>>,
}

impl Parse for If {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        expect!(TokenType::IfKeyword, ctx);

        ctx.save();

        let expr = try_or_restore!(Expression::parse(ctx), ctx);

        let mut is_multi = true;

        if ctx.cur_tok.t == TokenType::ThenKeyword {
            is_multi = false;

            ctx.consume();
        }

        let body = try_or_restore!(Body::parse(ctx), ctx);

        // in case of single line body
        if !is_multi || ctx.cur_tok.t == TokenType::EOL {
            expect!(TokenType::EOL, ctx);
        }

        let next = ctx.lexer.seek(1);

        if next.t != TokenType::ElseKeyword {
            ctx.save_pop();

            return Ok(If {
                predicat: expr,
                body,
                else_: None,
            });
        }

        expect_or_restore!(TokenType::Indent(ctx.block_indent), ctx);

        expect_or_restore!(TokenType::ElseKeyword, ctx);

        let else_ = try_or_restore!(Else::parse(ctx), ctx);

        ctx.save_pop();

        Ok(If {
            predicat: expr,
            body,
            else_: Some(Box::new(else_)),
        })
    }
}
