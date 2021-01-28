use crate::parser::macros::*;
use crate::Error;
use crate::Parser;

use crate::ast::Parse;
use crate::ast::{Argument, Arguments};

// impl ConstraintGen for SecondaryExpr {
//     fn constrain(&self, ctx: &mut InferBuilder) -> TypeId {
//         match self {
//             SecondaryExpr::Arguments(args) => args.constrain_vec(ctx),
//         };

//         1
//     }
//     fn constrain_vec(&self, ctx: &mut InferBuilder) -> Vec<TypeId> {
//         // println!("Constraint: SecondaryExpr");

//         match self {
//             SecondaryExpr::Arguments(args) => args.constrain_vec(ctx),
//         }
//     }
// }
