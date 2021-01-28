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

impl Parse for SecondaryExpr {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        // if let Ok(idx) = Self::index(ctx) {
        //     return Ok(SecondaryExpr::Index(idx));
        // }

        // if let Ok(sel) = Selector::parse(ctx) {
        //     return Ok(SecondaryExpr::Selector(sel));
        // }

        if let Ok(args) = Arguments::parse(ctx) {
            return Ok(SecondaryExpr::Arguments(args));
        }

        self::error!("Expected secondary".to_string(), ctx);
    }
}
