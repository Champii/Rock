use crate::infer::*;
use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::ast_print::*;
use crate::ast::Parse;
// use crate::ast::PrimitiveType;
// use crate::ast::Type;
// use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
// use crate::type_checker::TypeInferer;

use llvm_sys::core::LLVMConstInt;
use llvm_sys::core::LLVMInt1Type;
use llvm_sys::core::LLVMInt32Type;
use llvm_sys::LLVMValue;

use crate::error;

use super::{Identity, PrimitiveType, Type};

// use super::helper::Leaf;

#[derive(Debug, Clone)]
pub enum LiteralKind {
    Number(i64),
    String(String),
    Bool(u64),
}

impl AstPrint for LiteralKind {
    fn print(&self, ctx: &mut AstPrintContext) {
        let indent = String::from("  ").repeat(ctx.indent());

        match self {
            Self::Number(n) => println!("{}Number({})", indent, n),
            Self::String(s) => println!("{}String({})", indent, s),
            Self::Bool(b) => println!("{}Boolean({})", indent, b),
        }
    }
}

impl Parse for LiteralKind {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if let TokenType::Number(num) = ctx.cur_tok().t {
            ctx.consume();

            return Ok(LiteralKind::Number(num));
        }

        if let TokenType::Bool(b) = ctx.cur_tok().t {
            ctx.consume();

            let v = if b { 1 } else { 0 };

            return Ok(LiteralKind::Bool(v));
        }

        if let TokenType::String(s) = ctx.cur_tok().t.clone() {
            ctx.consume();

            return Ok(LiteralKind::String(s.clone()));
        }

        error!("Expected literal".to_string(), ctx);
    }
}

#[derive(Debug, Clone)]
pub struct Literal {
    pub kind: LiteralKind,
    pub identity: Identity,
}

impl Parse for Literal {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;

        Ok(Self {
            kind: LiteralKind::parse(ctx)?,
            identity: Identity::new(token_id),
        })
    }
}

impl Annotate for Literal {
    fn annotate(&self, ctx: &mut InferBuilder) {
        match &self.kind {
            LiteralKind::Number(n) => {
                ctx.new_type_solved(self.identity.clone(), Type::Primitive(PrimitiveType::Int64))
            }
            LiteralKind::String(s) => ctx.new_type_solved(
                self.identity.clone(),
                Type::Primitive(PrimitiveType::String(s.len())),
            ),
            LiteralKind::Bool(b) => {
                ctx.new_type_solved(self.identity.clone(), Type::Primitive(PrimitiveType::Bool))
            }
        }
    }
}

impl AstPrint for Literal {
    fn print(&self, ctx: &mut AstPrintContext) {
        self.kind.print(ctx);
    }
}
impl ConstraintGen for Literal {
    fn constrain(&self, ctx: &mut InferBuilder) -> TypeId {
        ctx.get_type_id(self.identity.clone()).unwrap()
    }
}
// impl Leaf for Literal {}
// annotate!(Literal, []);

// impl TypeInferer for Literal {
//     fn infer(&mut self, _ctx: &mut Context) -> Result<TypeInfer, Error> {
//         trace!("Literal ({:?})", self);

//         match &self {
//             Literal::Number(_) => Ok(Some(Type::Primitive(PrimitiveType::Int32))),
//             Literal::String(s) => Ok(Some(Type::Primitive(PrimitiveType::String(s.len())))),
//             Literal::Bool(_) => Ok(Some(Type::Primitive(PrimitiveType::Bool))),
//         }
//     }
// }

// impl IrBuilder for Literal {
//     fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
//         match self {
//             Literal::Number(num) => {
//                 let sign = if *num < 0 { 1 } else { 0 };
//                 let nb: u64 = (*num) as u64;
//                 unsafe { Some(LLVMConstInt(LLVMInt32Type(), nb, sign)) }
//             }
//             Literal::String(s) => s.build(context),
//             Literal::Bool(b) => unsafe { Some(LLVMConstInt(LLVMInt1Type(), b.clone(), 0)) },
//         }
//     }
// }
