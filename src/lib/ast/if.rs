use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Body;
use crate::ast::Else;
use crate::ast::Expression;
use crate::ast::Parse;
// use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
// use crate::type_checker::TypeInferer;

use llvm::LLVMIntPredicate;
use llvm_sys::core::LLVMAppendBasicBlockInContext;
use llvm_sys::core::LLVMBuildBr;
use llvm_sys::core::LLVMBuildCondBr;
use llvm_sys::core::LLVMBuildICmp;
use llvm_sys::core::LLVMConstInt;
use llvm_sys::core::LLVMGetLastBasicBlock;
use llvm_sys::core::LLVMGetLastFunction;
use llvm_sys::core::LLVMMoveBasicBlockAfter;
use llvm_sys::core::LLVMPositionBuilderAtEnd;
use llvm_sys::core::LLVMTypeOf;
use llvm_sys::LLVMValue;

use crate::generator::Generate;
use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct If {
    pub predicat: Expression,
    pub body: Body,
    pub else_: Option<Box<Else>>,
}

impl Parse for If {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        expect!(TokenType::IfKeyword, ctx);

        ctx.save();

        let expr = try_or_restore!(Expression::parse(ctx), ctx);

        let mut is_multi = true;

        if ctx.cur_tok().t == TokenType::ThenKeyword {
            is_multi = false;

            ctx.consume();
        }

        let body = try_or_restore!(Body::parse(ctx), ctx);

        // in case of single line body
        if !is_multi || ctx.cur_tok().t == TokenType::EOL {
            expect!(TokenType::EOL, ctx);
        }

        let next = ctx.seek(1);

        if next.t != TokenType::ElseKeyword {
            ctx.save_pop();

            return Ok(If {
                predicat: expr,
                body,
                else_: None,
            });
        }

        expect_or_restore!(TokenType::Indent(ctx.block_indent), ctx);

        expect_or_restore!(TokenType::ElseKeyword, ctx);

        let else_ = try_or_restore!(Else::parse(ctx), ctx);

        ctx.save_pop();

        Ok(If {
            predicat: expr,
            body,
            else_: Some(Box::new(else_)),
        })
    }
}

// impl TypeInferer for If {
//     fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
//         trace!("If");

//         self.body.infer(ctx)
//     }
// }

// impl Generate for If {
//     fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
//         self.body.generate(ctx)
//     }
// }

// impl IrBuilder for If {
//     fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
//         unsafe {
//             let func = LLVMGetLastFunction(context.module);

//             let base_block = LLVMGetLastBasicBlock(func);

//             let predicat = self.predicat.build(context).unwrap();

//             let icmp = LLVMBuildICmp(
//                 context.builder,
//                 LLVMIntPredicate::LLVMIntNE,
//                 predicat,
//                 LLVMConstInt(LLVMTypeOf(predicat), 0, 0),
//                 "\0".as_ptr() as *const _,
//             );

//             let if_body = LLVMAppendBasicBlockInContext(
//                 context.context,
//                 func,
//                 b"then\0".as_ptr() as *const _,
//             );

//             LLVMPositionBuilderAtEnd(context.builder, if_body);

//             let body = self.body.build(context).unwrap();

//             let final_block = LLVMAppendBasicBlockInContext(
//                 context.context,
//                 func,
//                 "final\0".as_ptr() as *const _,
//             );

//             LLVMBuildBr(context.builder, final_block);

//             let res_block = if let Some(else_) = self.else_.clone() {
//                 LLVMPositionBuilderAtEnd(context.builder, final_block);

//                 let else_body = LLVMAppendBasicBlockInContext(
//                     context.context,
//                     func,
//                     b"else\0".as_ptr() as *const _,
//                 );

//                 LLVMPositionBuilderAtEnd(context.builder, else_body);

//                 else_.build(context).unwrap();

//                 LLVMBuildBr(context.builder, final_block);

//                 LLVMMoveBasicBlockAfter(final_block, else_body);

//                 else_body
//             } else {
//                 final_block
//             };

//             LLVMPositionBuilderAtEnd(context.builder, base_block);

//             LLVMBuildCondBr(context.builder, icmp, if_body, res_block);

//             LLVMPositionBuilderAtEnd(context.builder, final_block);

//             Some(body)
//         }
//     }
// }
