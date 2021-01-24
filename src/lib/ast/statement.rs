use super::Identity;
use crate::error;
use crate::infer::*;
use crate::Error;
use crate::Parser;

use crate::ast::Expression;
use crate::ast::If;
use crate::ast::Parse;

#[derive(Debug, Clone)]
pub enum StatementKind {
    If(If),
    // For(For),
    Expression(Expression),
    // Assignation(Assignation),
}

visitable_constraint_enum!(
    StatementKind,
    ConstraintGen,
    constrain,
    InferBuilder,
    [Expression(x)]
);

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: Box<StatementKind>,
    pub identity: Identity,
}

visitable_constraint_class!(Statement, ConstraintGen, constrain, InferBuilder, [kind]);

impl Parse for Statement {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok_id;

        let kind = Box::new(if let Ok(if_) = If::parse(ctx) {
            StatementKind::If(if_)
        // } else if let Ok(for_) = For::parse(ctx) {
        //     StatementKind::For(for_)
        // } else if let Ok(assign) = Assignation::parse(ctx) {
        //     StatementKind::Assignation(assign)
        } else if let Ok(expr) = Expression::parse(ctx) {
            StatementKind::Expression(expr)
        } else {
            error!("Expected statement".to_string(), ctx);
        });

        Ok(Statement {
            kind,
            identity: Identity::new(token),
        })
    }
}
