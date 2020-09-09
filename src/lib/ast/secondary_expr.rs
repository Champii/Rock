use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Expression;
use crate::ast::Parse;
use crate::ast::Selector;
use crate::ast::TypeInfer;
use crate::ast::{Argument, Arguments};

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use llvm_sys::core::LLVMBuildCall;
use llvm_sys::core::LLVMBuildGEP;
use llvm_sys::core::LLVMConstInt;
use llvm_sys::core::LLVMInt32Type;
use llvm_sys::LLVMValue;

use crate::generator::Generate;
use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub enum SecondaryExpr {
    Selector(Selector), // . Identifier  // u8 is the attribute index in struct // option<Type> is the class type if needed // RealFullName
    Arguments(Vec<Argument>), // (Expr, Expr, ...)
    Index(Box<Expression>), // [Expr]
}

impl SecondaryExpr {
    fn index(ctx: &mut Parser) -> Result<Box<Expression>, Error> {
        ctx.save();

        expect_or_restore!(TokenType::OpenArray, ctx);

        let expr = try_or_restore!(Expression::parse(ctx), ctx);

        expect_or_restore!(TokenType::CloseArray, ctx);

        ctx.save_pop();

        return Ok(Box::new(expr));
    }
}

impl Parse for SecondaryExpr {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if let Ok(idx) = Self::index(ctx) {
            return Ok(SecondaryExpr::Index(idx));
        }

        if let Ok(sel) = Selector::parse(ctx) {
            return Ok(SecondaryExpr::Selector(sel));
        }

        if let Ok(args) = Arguments::parse(ctx) {
            return Ok(SecondaryExpr::Arguments(args));
        }

        self::error!("Expected secondary".to_string(), ctx);
    }
}

impl TypeInferer for SecondaryExpr {
    fn infer(&mut self, _ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            _ => Err(Error::new_empty()),
        }
    }
}

impl Generate for SecondaryExpr {
    fn generate(&mut self, _ctx: &mut Context) -> Result<(), Error> {
        match self {
            _ => Err(Error::new_empty()),
        }
    }
}

impl SecondaryExpr {
    pub fn build_with(
        &self,
        context: &mut IrContext,
        op: *mut LLVMValue,
    ) -> Option<*mut LLVMValue> {
        match self {
            SecondaryExpr::Arguments(args) => {
                let mut res = vec![];

                for arg in args {
                    res.push(arg.build(context).unwrap());
                }

                unsafe {
                    Some(LLVMBuildCall(
                        context.builder,
                        op,
                        res.as_mut_ptr(),
                        res.len() as u32,
                        b"\0".as_ptr() as *const _,
                    ))
                }
            }

            SecondaryExpr::Index(expr) => {
                let idx = expr.build(context).unwrap();

                unsafe {
                    let mut indices = [idx];

                    let ptr_elem = LLVMBuildGEP(
                        context.builder,
                        op,
                        indices.as_mut_ptr(),
                        1,
                        b"\0".as_ptr() as *const _,
                    );

                    Some(ptr_elem)
                }
            }

            SecondaryExpr::Selector(sel) => unsafe {
                let zero = LLVMConstInt(LLVMInt32Type(), 0, 0);
                let idx = LLVMConstInt(LLVMInt32Type(), sel.class_offset as u64, 0);

                let mut indices = [zero, idx];

                if let Some(f) = context.functions.get(sel.full_name.clone()) {
                    return Some(f);
                }

                let ptr_elem = LLVMBuildGEP(
                    context.builder,
                    op,
                    indices.as_mut_ptr(),
                    2,
                    b"\0".as_ptr() as *const _,
                );

                Some(ptr_elem)
            },
        }
    }
}
