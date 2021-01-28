use crate::parser::macros::*;
use crate::Error;
use crate::Parser;
use crate::TokenType;

use super::Identity;
use crate::ast::argument_decl::ArgumentsDecl;
use crate::ast::helper::*;
use crate::ast::ArgumentDecl;
use crate::ast::Body;
use crate::ast::Identifier;
use crate::ast::Parse;

//     fn annotate(&self, ctx: &mut InferBuilder) {
//         let _args = self.arguments.annotate(ctx);
//         let _ret = self.body.annotate(ctx);

//         ctx.new_named_annotation((*self.name).clone(), self.identity.clone());
//     }
// }

// impl ConstraintGen for FunctionDecl {
//     fn constrain(&self, ctx: &mut InferBuilder) -> TypeId {
//         // println!("Constraint: FunctionDecl");

//         let args = self.arguments.constrain_vec(ctx);
//         let body_type = self.body.constrain(ctx);

//         let self_type_id = ctx.get_type_id(self.identity.clone()).unwrap();

//         ctx.solve_type(
//             self.identity.clone(),
//             Type::FuncType(FuncType::new((*self.name).clone(), args, body_type)),
//         );

//         ctx.add_constraint(Constraint::Eq(self_type_id, body_type));

//         self_type_id
//     }
// }
