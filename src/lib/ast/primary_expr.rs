use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;
use crate::ast::SecondaryExpr;
use crate::ast::{Operand, OperandKind};

// impl ConstraintGen for PrimaryExpr {
//     fn constrain(&self, ctx: &mut InferBuilder) -> TypeId {
//         // println!("Constraint: PrimaryExpr");

//         match self {
//             PrimaryExpr::PrimaryExpr(op, secs) => {
//                 let args_type_ids = secs
//                     .iter()
//                     .map(|x| x.constrain_vec(ctx))
//                     .flatten()
//                     .collect::<Vec<TypeId>>();

//                 if let OperandKind::Identifier(id) = &op.kind {
//                     if let Some(f_id) = ctx.get_named_type_id((*id).to_string()) {
//                         if let Some(f_type) = ctx.get_type(f_id) {
//                             if let Type::FuncType(f) = f_type {
//                                 for sec in secs {
//                                     match sec {
//                                         SecondaryExpr::Arguments(args) => {
//                                             let mut i = 0;

//                                             for _arg in args {
//                                                 ctx.add_constraint(Constraint::Eq(
//                                                     *f.arguments.get(i).unwrap(),
//                                                     *args_type_ids.get(i).unwrap(),
//                                                 ));

//                                                 i += 1;
//                                             }
//                                         }
//                                     };
//                                 }

//                                 op.constrain(ctx);

//                                 return f.ret;
//                             }
//                         }
//                     }
//                 }

//                 op.constrain(ctx)
//             }
//         }
//     }
// }
