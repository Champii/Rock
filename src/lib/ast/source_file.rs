use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;
use crate::ast::TopLevel;
use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;
use llvm_sys::LLVMValue;

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub top_levels: Vec<TopLevel>,
}

impl Parse for SourceFile {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut top_levels = vec![];

        while ctx.cur_tok.t != TokenType::EOF {
            top_levels.push(TopLevel::parse(ctx)?);
        }

        expect!(TokenType::EOF, ctx);

        Ok(SourceFile { top_levels })
    }
}

impl TypeInferer for SourceFile {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("SourceFile");

        let mut last = Err(Error::new_empty());

        let mut top_level_methods = vec![];

        for top in &mut self.top_levels {
            last = Ok(top.infer(ctx)?);
            match top {
                TopLevel::Class(class) => {
                    for method in &class.methods {
                        top_level_methods.push(method.clone());
                    }
                }
                _ => (),
            }
        }

        for method in top_level_methods {
            self.top_levels.push(TopLevel::Function(method));
        }

        last
    }
}

impl IrBuilder for SourceFile {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        for top in &self.top_levels {
            top.build(context);
        }

        None
    }
}
