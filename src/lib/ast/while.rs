use crate::Error;
use crate::Parser;

use crate::ast::Body;
use crate::ast::Expression;
use crate::ast::Parse;
use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use llvm::LLVMIntPredicate;
use llvm_sys::core::LLVMAppendBasicBlockInContext;
use llvm_sys::core::LLVMBuildCondBr;
use llvm_sys::core::LLVMBuildICmp;
use llvm_sys::core::LLVMConstInt;
use llvm_sys::core::LLVMGetLastBasicBlock;
use llvm_sys::core::LLVMGetLastFunction;
use llvm_sys::core::LLVMPositionBuilderAtEnd;
use llvm_sys::core::LLVMTypeOf;
use llvm_sys::LLVMValue;

use crate::try_or_restore;

#[derive(Debug, Clone)]
pub struct While {
    pub predicat: Expression,
    pub body: Body,
}

impl Parse for While {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        ctx.save();

        let predicat = try_or_restore!(Expression::parse(ctx), ctx);

        let body = try_or_restore!(Body::parse(ctx), ctx);

        ctx.save_pop();

        Ok(While { predicat, body })
    }
}

impl TypeInferer for While {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("While");

        self.body.infer(ctx)
    }
}

impl IrBuilder for While {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        unsafe {
            let func = LLVMGetLastFunction(context.module);

            let base_block = LLVMGetLastBasicBlock(func);

            let predicat = self.predicat.build(context).unwrap();

            let icmp = LLVMBuildICmp(
                context.builder,
                LLVMIntPredicate::LLVMIntNE,
                predicat,
                LLVMConstInt(LLVMTypeOf(predicat), 0, 0),
                "\0".as_ptr() as *const _,
            );

            let for_body = LLVMAppendBasicBlockInContext(
                context.context,
                func,
                b"then\0".as_ptr() as *const _,
            );

            LLVMPositionBuilderAtEnd(context.builder, for_body);

            let body = self.body.build(context).unwrap();

            let res_block = LLVMAppendBasicBlockInContext(
                context.context,
                func,
                "final\0".as_ptr() as *const _,
            );

            let predicat2 = self.predicat.build(context).unwrap();

            let icmp2 = LLVMBuildICmp(
                context.builder,
                LLVMIntPredicate::LLVMIntNE,
                predicat2,
                LLVMConstInt(LLVMTypeOf(predicat), 0, 0),
                "\0".as_ptr() as *const _,
            );

            LLVMBuildCondBr(context.builder, icmp2, for_body, res_block);

            LLVMPositionBuilderAtEnd(context.builder, base_block);

            LLVMBuildCondBr(context.builder, icmp, for_body, res_block);

            LLVMPositionBuilderAtEnd(context.builder, res_block);

            Some(body)
        }
    }
}
