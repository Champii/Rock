#[macro_use]
use crate::infer::*;
use crate::Parser;
use crate::TokenType;
use crate::{token::TokenId, Error};

use crate::ast::ast_print::*;
use crate::ast::Parse;
use crate::ast::TopLevel;

// use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
// use crate::type_checker::TypeInferer;
use llvm_sys::LLVMValue;

use crate::generator::Generate;
use crate::parser::macros::*;

use super::identity::Identity;

#[derive(Debug, Clone, Default)]
pub struct SourceFile {
    pub top_levels: Vec<TopLevel>,
    pub identity: Identity,
}

// annotate!(struct, SourceFile, [top_levels]);

impl ConstraintGen for SourceFile {
    fn constrain(&self, ctx: &mut InferBuilder) -> TypeId {
        println!("Constraint: SourceFile");

        self.top_levels.constrain_vec(ctx);

        ctx.remove_node_id(self.identity.clone());

        0
    }
}

impl Parse for SourceFile {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut top_levels = vec![];

        while ctx.cur_tok().t != TokenType::EOF {
            top_levels.push(TopLevel::parse(ctx)?);
        }

        expect!(TokenType::EOF, ctx);

        Ok(SourceFile {
            top_levels,
            identity: Identity::new(0),
        })
    }
}

// impl Annotate for SourceFile {
//     fn annotate(&self, ctx: &mut InferBuilder) {
//         //
//     }
// }

// impl TypeInferer for SourceFile {
//     fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
//         trace!("SourceFile");

//         let mut last = Err(Error::new_empty());

//         let mut top_level_methods = vec![];

//         for top in &mut self.top_levels {
//             last = Ok(top.infer(ctx)?);
//             match top {
//                 TopLevel::Class(class) => {
//                     for method in &class.methods {
//                         top_level_methods.push(method.clone());
//                     }
//                 }
//                 _ => (),
//             }
//         }

//         for method in top_level_methods {
//             self.top_levels.push(TopLevel::Function(method));
//         }

//         last
//     }
// }

// impl Generate for SourceFile {
//     fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
//         for top in &mut self.top_levels {
//             top.generate(ctx)?;
//         }

//         Ok(())
//     }
// }

// impl IrBuilder for SourceFile {
//     fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
//         for top in &self.top_levels {
//             top.build(context);
//         }

//         None
//     }
// }
