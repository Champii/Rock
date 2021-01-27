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

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: Identifier,
    pub arguments: Vec<ArgumentDecl>,
    pub body: Body,
    pub identity: Identity,
}

impl Parse for FunctionDecl {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut arguments = vec![];

        let token = ctx.cur_tok_id;

        ctx.save();

        let name = try_or_restore!(Identifier::parse(ctx), ctx);

        if TokenType::OpenParens == ctx.cur_tok().t
            || TokenType::Identifier(ctx.cur_tok().txt.clone()) == ctx.cur_tok().t
        {
            // manage error and restore here
            arguments = ArgumentsDecl::parse(ctx)?;
        }

        expect_or_restore!(TokenType::Equal, ctx);

        let body = try_or_restore!(Body::parse(ctx), ctx);

        ctx.save_pop();

        Ok(FunctionDecl {
            name,
            arguments,
            body,
            identity: Identity::new(token),
        })
    }
}

generate_has_name!(FunctionDecl);

// impl Annotate for FunctionDecl {
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
