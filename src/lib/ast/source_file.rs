use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;
use crate::ast::TopLevel;

use super::identity::Identity;
use crate::infer::*;
use crate::parser::macros::*;

#[derive(Debug, Clone, Default)]
pub struct SourceFile {
    pub top_levels: Vec<TopLevel>,
    pub identity: Identity,
}

impl ConstraintGen for SourceFile {
    fn constrain(&self, ctx: &mut InferBuilder) -> TypeId {
        // println!("Constraint: SourceFile");

        self.top_levels.constrain_vec(ctx);

        ctx.remove_node_id(self.identity.clone());

        0
    }
}

impl Parse for SourceFile {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut top_levels = vec![];

        while ctx.cur_tok().t != TokenType::EOF {
            top_levels.push(TopLevel::parse(ctx)?);
        }

        expect!(TokenType::EOF, ctx);

        Ok(SourceFile {
            top_levels,
            identity: Identity::new(0),
        })
    }
}
