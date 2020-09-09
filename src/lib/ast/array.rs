use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Expression;
use crate::ast::Parse;
use crate::ast::PrimitiveType;
use crate::ast::Type;
use crate::ast::TypeInfer;

use crate::codegen::get_type;
use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use llvm_sys::core::LLVMArrayType;
use llvm_sys::core::LLVMBuildAlloca;
use llvm_sys::core::LLVMBuildBitCast;
use llvm_sys::core::LLVMBuildGEP;
use llvm_sys::core::LLVMBuildStore;
use llvm_sys::core::LLVMConstInt;
use llvm_sys::core::LLVMGetElementType;
use llvm_sys::core::LLVMInt32Type;
use llvm_sys::LLVMValue;

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct Array {
    pub items: Vec<Expression>,
    pub t: Option<Type>,
}

impl Parse for Array {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        ctx.save();

        expect_or_restore!(TokenType::OpenArray, ctx);

        let mut items = vec![];

        while ctx.cur_tok.t != TokenType::CloseArray {
            let item = try_or_restore!(Expression::parse(ctx), ctx);

            items.push(item);

            if ctx.cur_tok.t != TokenType::Coma && ctx.cur_tok.t != TokenType::CloseArray {
                ctx.restore();
            }

            if ctx.cur_tok.t == TokenType::Coma {
                ctx.consume();
            }
        }

        expect_or_restore!(TokenType::CloseArray, ctx);

        ctx.save_pop();

        Ok(Array { items, t: None })
    }
}

impl TypeInferer for Array {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("Array ({:?})", self);

        let mut last = None;

        for item in &mut self.items {
            let t = item.infer(ctx)?;

            if let None = last {
                last = t.clone();
            }

            if last.clone().unwrap().get_name() != t.clone().unwrap().get_name() {
                // TODO: type error
                return Err(Error::new_empty());
            }
        }

        self.t = Some(Type::Primitive(PrimitiveType::Array(
            Box::new(last.unwrap()),
            self.items.len(),
        )));

        Ok(self.t.clone())
    }
}

impl IrBuilder for Array {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        let mut res = vec![];

        for item in &self.items {
            res.push(item.build(context).unwrap());
        }

        let t = get_type(Box::new(self.t.clone().unwrap()), context);

        unsafe {
            let pointer = LLVMBuildAlloca(
                context.builder,
                LLVMArrayType(LLVMGetElementType(t), res.len() as u32),
                b"\0".as_ptr() as *const _,
            );

            let zero = LLVMConstInt(LLVMInt32Type(), 0, 0);
            let mut indices = [zero, zero];

            let ptr_elem = LLVMBuildGEP(
                context.builder,
                pointer,
                indices.as_mut_ptr(),
                2,
                b"\0".as_ptr() as *const _,
            );

            let mut i = 0;

            for item in res {
                let idx = LLVMConstInt(LLVMInt32Type(), i, 0);
                let mut indices = [zero, idx];

                let ptr_elem = LLVMBuildGEP(
                    context.builder,
                    pointer,
                    indices.as_mut_ptr(),
                    2,
                    b"\0".as_ptr() as *const _,
                );

                LLVMBuildStore(context.builder, item, ptr_elem);

                i += 1;
            }

            let ptr8 = LLVMBuildBitCast(context.builder, ptr_elem, t, b"\0".as_ptr() as *const _);

            Some(ptr8)
        }
    }
}
