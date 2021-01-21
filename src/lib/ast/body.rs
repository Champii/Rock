use crate::Parser;
use crate::TokenType;
use crate::{token::TokenId, Error};

use crate::ast::ast_print::*;
use crate::ast::Parse;
use crate::ast::Statement;
// use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
// use crate::type_checker::TypeInferer;

use crate::generator::Generate;
use llvm_sys::LLVMValue;

#[derive(Debug, Clone)]
pub struct Body {
    pub stmt: Statement,
    pub token: TokenId,
}

derive_print!(Body, [stmt]);

impl Parse for Body {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let stmt = Statement::parse(ctx)?;

        Ok(Body {
            token: stmt.token,
            stmt,
        })
    }
}

// impl TypeInferer for Body {
//     fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
//         trace!("Body");

//         let mut last = Err(Error::new_empty());

//         for stmt in &mut self.stmts {
//             last = Ok(stmt.infer(ctx)?);
//         }

//         last
//     }
// }

// impl Generate for Body {
//     fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
//         for stmt in &mut self.stmts {
//             stmt.generate(ctx)?;
//         }

//         Ok(())
//     }
// }

// impl IrBuilder for Body {
//     fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
//         let mut last = None;

//         for stmt in &self.stmts {
//             last = stmt.build(context);
//         }

//         last
//     }
// }
