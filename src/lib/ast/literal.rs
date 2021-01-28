use crate::error;
use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;

use super::Identity;

impl Parse for LiteralKind {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if let TokenType::Number(num) = ctx.cur_tok().t {
            ctx.consume();

            return Ok(LiteralKind::Number(num));
        }

        if let TokenType::Bool(b) = ctx.cur_tok().t {
            ctx.consume();

            let v = if b { 1 } else { 0 };

            return Ok(LiteralKind::Bool(v));
        }

        if let TokenType::String(s) = ctx.cur_tok().t.clone() {
            ctx.consume();

            return Ok(LiteralKind::String(s.clone()));
        }

        error!("Expected literal".to_string(), ctx);
    }
}

impl Parse for Literal {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;

        Ok(Self {
            kind: LiteralKind::parse(ctx)?,
            identity: Identity::new(token_id),
        })
    }
}

// impl Annotate for Literal {
//     fn annotate(&self, ctx: &mut InferBuilder) {
//         match &self.kind {
//             LiteralKind::Number(_n) => {
//                 ctx.new_type_solved(self.identity.clone(), Type::Primitive(PrimitiveType::Int64))
//             }
//             LiteralKind::String(s) => ctx.new_type_solved(
//                 self.identity.clone(),
//                 Type::Primitive(PrimitiveType::String(s.len())),
//             ),
//             LiteralKind::Bool(_b) => {
//                 ctx.new_type_solved(self.identity.clone(), Type::Primitive(PrimitiveType::Bool))
//             }
//         }
//     }
// }

// impl ConstraintGen for Literal {
//     fn constrain(&self, ctx: &mut InferBuilder) -> TypeId {
//         ctx.get_type_id(self.identity.clone()).unwrap()
//     }
// }
