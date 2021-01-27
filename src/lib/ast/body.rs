use super::Identity;
use crate::Error;
use crate::Parser;

use crate::ast::Parse;
use crate::ast::Statement;

#[derive(Debug, Clone)]
pub struct Body {
    pub stmt: Statement,
    pub identity: Identity,
}

impl Parse for Body {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let stmt = Statement::parse(ctx)?;

        Ok(Body {
            identity: Identity::new(stmt.identity.token_id),
            stmt,
        })
    }
}
