use crate::Error;
use crate::Parser;
use crate::Token;

use crate::ast::Assignation;
use crate::ast::Expression;
use crate::ast::For;
use crate::ast::If;
use crate::ast::Parse;
use crate::ast::TypeInfer;

use crate::error;

#[derive(Debug, Clone)]
pub enum StatementKind {
    If(If),
    For(For),
    Expression(Expression),
    Assignation(Assignation),
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: StatementKind,
    pub t: TypeInfer,
    pub token: Token,
}

impl Parse for Statement {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok.clone();

        let kind = if let Ok(if_) = If::parse(ctx) {
            StatementKind::If(if_)
        } else if let Ok(for_) = For::parse(ctx) {
            StatementKind::For(for_)
        } else if let Ok(assign) = Assignation::parse(ctx) {
            StatementKind::Assignation(assign)
        } else if let Ok(expr) = Expression::parse(ctx) {
            StatementKind::Expression(expr)
        } else {
            error!("Expected statement".to_string(), ctx);
        };

        Ok(Statement {
            kind,
            t: None,
            token,
        })
    }
}
