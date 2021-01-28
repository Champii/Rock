use super::Identity;
use crate::parser::macros::*;
use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;

// impl Annotate for ArgumentDecl {
//     fn annotate(&self, ctx: &mut InferBuilder) {
//         ctx.new_named_annotation(self.name.clone(), self.identity.clone());
//     }
// }

// impl ConstraintGen for ArgumentDecl {
//     fn constrain(&self, ctx: &mut InferBuilder) -> TypeId {
//         ctx.get_type_id(self.identity.clone()).unwrap()
//     }
// }
