use super::Identity;
use crate::infer::*;
use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::FunctionDecl;
use crate::ast::Parse;

#[derive(Debug, Clone)]
pub enum TopLevelKind {
    Function(FunctionDecl),
}

visitable_constraint_enum!(
    TopLevelKind,
    ConstraintGen,
    constrain,
    InferBuilder,
    [Function(x)]
);

#[derive(Debug, Clone)]
pub struct TopLevel {
    pub kind: TopLevelKind,
    pub identity: Identity,
}

visitable_constraint_class!(TopLevel, ConstraintGen, constrain, InferBuilder, [kind]);

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
