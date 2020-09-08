use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;

use crate::error;

#[derive(Debug, Clone)]
pub enum Literal {
    Number(u64),
    String(String),
    Bool(u64),
}

impl Parse for Literal {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if let TokenType::Number(num) = ctx.cur_tok.t {
            ctx.consume();

            return Ok(Literal::Number(num));
        }

        if let TokenType::Bool(b) = ctx.cur_tok.t {
            ctx.consume();

            let v = if b { 1 } else { 0 };

            return Ok(Literal::Bool(v));
        }

        if let TokenType::String(s) = ctx.cur_tok.t.clone() {
            ctx.consume();

            return Ok(Literal::String(s.clone()));
        }

        error!("Expected literal".to_string(), ctx);
    }
}
