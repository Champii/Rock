use crate::try_or_restore_and;
use crate::Error;
use crate::Parser;

use crate::ast::Identity;
use crate::ast::Operator;
use crate::ast::Parse;
use crate::ast::UnaryExpr;

#[derive(Debug, Clone)]
pub enum ExpressionKind {
    BinopExpr(UnaryExpr, Operator, Box<Expression>),
    UnaryExpr(UnaryExpr),
}

#[derive(Debug, Clone)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub identity: Identity,
}

impl Expression {
    pub fn is_literal(&self) -> bool {
        match &self.kind {
            ExpressionKind::UnaryExpr(unary) => unary.is_literal(),
            _ => false,
        }
    }

    pub fn is_identifier(&self) -> bool {
        match &self.kind {
            ExpressionKind::UnaryExpr(unary) => unary.is_identifier(),
            _ => false,
        }
    }

    pub fn get_identifier(&self) -> Option<String> {
        match &self.kind {
            ExpressionKind::UnaryExpr(unary) => unary.get_identifier(),
            _ => None,
        }
    }
}

impl Parse for Expression {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok_id;

        let left = UnaryExpr::parse(ctx)?;

        let mut res = Expression {
            kind: ExpressionKind::UnaryExpr(left.clone()),
            identity: Identity::new(token),
        };

        ctx.save();

        let op = try_or_restore_and!(Operator::parse(ctx), Ok(res), ctx);

        let right = try_or_restore_and!(Expression::parse(ctx), Ok(res), ctx);

        ctx.save_pop();

        res.kind = ExpressionKind::BinopExpr(left, op, Box::new(right));

        Ok(res)
    }
}
