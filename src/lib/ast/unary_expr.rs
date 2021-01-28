use crate::try_or_restore;
use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::OperandKind;
use crate::ast::Operator;
use crate::ast::Parse;
use crate::ast::PrimaryExpr;

// impl ConstraintGen for UnaryExpr {
//     fn constrain(&self, ctx: &mut InferBuilder) -> TypeId {
//         // println!("Constraint: UnaryExpr");

//         match self {
//             UnaryExpr::PrimaryExpr(p) => p.constrain(ctx),
//             UnaryExpr::UnaryExpr(op, unary) => {
//                 unary.constrain(ctx);

//                 op.constrain(ctx)
//             }
//        //  }
// //     }
// // }
