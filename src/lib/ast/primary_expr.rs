use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;
use crate::ast::SecondaryExpr;
use crate::ast::{Operand, OperandKind};

#[derive(Debug, Clone)]
pub enum PrimaryExpr {
    PrimaryExpr(Operand, Vec<SecondaryExpr>),
}

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

impl PrimaryExpr {
    pub fn has_secondaries(&self) -> bool {
        match self {
            PrimaryExpr::PrimaryExpr(_, vec) => vec.len() > 0,
        }
    }

    pub fn get_identifier(&self) -> Option<String> {
        match self {
            PrimaryExpr::PrimaryExpr(op, _) => {
                if let OperandKind::Identifier(ident) = &op.kind {
                    Some(ident.name.clone())
                } else {
                    None
                }
            }
        }
    }
}

impl Parse for PrimaryExpr {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let operand = Operand::parse(ctx)?;

        let mut secondarys = vec![];

        if ctx.cur_tok().t == TokenType::Operator(ctx.cur_tok().txt.clone())
            || ctx.cur_tok().t == TokenType::Equal
        {
            return Ok(PrimaryExpr::PrimaryExpr(operand, secondarys));
        }

        while let Ok(second) = SecondaryExpr::parse(ctx) {
            secondarys.push(second);

            if ctx.cur_tok().t == TokenType::Operator(ctx.cur_tok().txt.clone())
                || ctx.cur_tok().t == TokenType::Equal
            {
                break;
            }
        }

        Ok(PrimaryExpr::PrimaryExpr(operand, secondarys))
    }
}
