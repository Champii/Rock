use super::Identity;
use crate::infer::*;
use crate::parser::macros::*;
use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Expression;
use crate::ast::Parse;

pub type Arguments = Vec<Argument>;

impl Parse for Arguments {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut res = vec![];

        ctx.save();

        loop {
            let arg = try_or_restore!(Argument::parse(ctx), ctx);

            res.push(arg);

            if TokenType::Coma != ctx.cur_tok().t {
                break;
            }

            ctx.consume();
        }

        ctx.save_pop();

        Ok(res)
    }
}

#[derive(Debug, Clone)]
pub struct Argument {
    pub arg: Expression,
    pub identity: Identity,
}

impl Parse for Argument {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok_id;

        Ok(Argument {
            arg: Expression::parse(ctx)?,
            identity: Identity::new(token),
        })
    }
}

visitable_constraint_class!(Argument, ConstraintGen, constrain, InferBuilder, [arg]);
