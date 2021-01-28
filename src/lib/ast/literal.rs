use crate::error;
use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;

use super::Identity;

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
