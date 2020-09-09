use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;
use crate::ast::PrimitiveType;
use crate::ast::Type;
use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use llvm_sys::core::LLVMConstInt;
use llvm_sys::core::LLVMInt1Type;
use llvm_sys::core::LLVMInt32Type;
use llvm_sys::LLVMValue;

use crate::error;

#[derive(Debug, Clone)]
pub enum Literal {
    Number(u64),
    String(String),
    Bool(u64),
}

impl Parse for Literal {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if let TokenType::Number(num) = ctx.cur_tok.t {
            ctx.consume();

            return Ok(Literal::Number(num));
        }

        if let TokenType::Bool(b) = ctx.cur_tok.t {
            ctx.consume();

            let v = if b { 1 } else { 0 };

            return Ok(Literal::Bool(v));
        }

        if let TokenType::String(s) = ctx.cur_tok.t.clone() {
            ctx.consume();

            return Ok(Literal::String(s.clone()));
        }

        error!("Expected literal".to_string(), ctx);
    }
}

impl TypeInferer for Literal {
    fn infer(&mut self, _ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("Literal ({:?})", self);

        match &self {
            Literal::Number(_) => Ok(Some(Type::Primitive(PrimitiveType::Int))),
            Literal::String(s) => Ok(Some(Type::Primitive(PrimitiveType::String(s.len())))),
            Literal::Bool(_) => Ok(Some(Type::Primitive(PrimitiveType::Bool))),
        }
    }
}

impl IrBuilder for Literal {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        match self {
            Literal::Number(num) => unsafe { Some(LLVMConstInt(LLVMInt32Type(), *num, 0)) },
            Literal::String(s) => s.build(context),
            Literal::Bool(b) => unsafe { Some(LLVMConstInt(LLVMInt1Type(), b.clone(), 0)) },
        }
    }
}
