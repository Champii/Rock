use super::Identity;
use crate::error;
use crate::Error;
use crate::Parser;

use crate::ast::Expression;
use crate::ast::If;
use crate::ast::Parse;

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
