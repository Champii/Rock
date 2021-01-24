use super::Identity;
use crate::infer::*;
use crate::Error;
use crate::Parser;

use crate::ast::Parse;
use crate::ast::Statement;

#[derive(Debug, Clone)]
pub struct Body {
    pub stmt: Statement,
    pub identity: Identity,
}

visitable_constraint_class!(Body, ConstraintGen, constrain, InferBuilder, [stmt]);

impl Parse for Body {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let stmt = Statement::parse(ctx)?;

        Ok(Body {
            identity: Identity::new(stmt.identity.token_id),
            stmt,
        })
    }
}
