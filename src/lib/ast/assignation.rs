use crate::Error;
use crate::Parser;
use crate::Token;
use crate::TokenType;

use crate::ast::Parse;
use crate::ast::PrimaryExpr;
use crate::ast::Statement;
use crate::ast::Type;

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct Assignation {
    pub name: PrimaryExpr,
    pub t: Option<Type>,
    pub value: Box<Statement>,
    pub token: Token,
}

impl Parse for Assignation {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        ctx.save();

        let name = try_or_restore!(PrimaryExpr::parse(ctx), ctx);

        let t = if ctx.cur_tok.t == TokenType::SemiColon {
            ctx.consume();

            Some(try_or_restore_expect!(
                Type::parse(ctx),
                TokenType::Type(ctx.cur_tok.txt.clone()),
                ctx
            ))
        } else {
            None
        };

        expect_or_restore!(TokenType::Equal, ctx);

        let stmt = try_or_restore!(Statement::parse(ctx), ctx);

        ctx.save_pop();

        Ok(Assignation {
            name,
            t,
            token: stmt.token.clone(),
            value: Box::new(stmt),
        })
    }
}
