use super::Identity;
use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::FunctionDecl;
use crate::parser::Parse;

impl Parse for TopLevel {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token = ctx.cur_tok_id;

        let kind = match ctx.cur_tok().t {
            _ => TopLevelKind::Function(FunctionDecl::parse(ctx)?),
        };

        while ctx.cur_tok().t == TokenType::EOL {
            ctx.consume();
        }

        Ok(TopLevel {
            kind,
            identity: Identity::new(token),
        })
    }
}
