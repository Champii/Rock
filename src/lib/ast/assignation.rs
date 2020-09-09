use crate::Error;
use crate::Parser;
use crate::Token;
use crate::TokenType;
use llvm_sys::core::LLVMBuildAlloca;
use llvm_sys::core::LLVMBuildStore;

use crate::ast::Parse;
use crate::ast::PrimaryExpr;
use crate::ast::Statement;
use crate::ast::Type;
use crate::ast::TypeInfer;

use crate::codegen::get_type;
use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use llvm_sys::LLVMValue;

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct Assignation {
    pub name: PrimaryExpr,
    pub t: Option<Type>,
    pub value: Box<Statement>,
    pub token: Token,
}

impl Parse for Assignation {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        ctx.save();

        let name = try_or_restore!(PrimaryExpr::parse(ctx), ctx);

        let t = if ctx.cur_tok.t == TokenType::SemiColon {
            ctx.consume();

            Some(try_or_restore_expect!(
                Type::parse(ctx),
                TokenType::Type(ctx.cur_tok.txt.clone()),
                ctx
            ))
        } else {
            None
        };

        expect_or_restore!(TokenType::Equal, ctx);

        let stmt = try_or_restore!(Statement::parse(ctx), ctx);

        ctx.save_pop();

        Ok(Assignation {
            name,
            t,
            token: stmt.token.clone(),
            value: Box::new(stmt),
        })
    }
}

impl TypeInferer for Assignation {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("Assignation ({:?})", self.token);

        // simple identifier
        if !self.name.has_secondaries() {
            let name = self.name.get_identifier().unwrap();
            let res = ctx.scopes.get(name.clone());

            if let Some(t) = res {
                let value_t = self.value.infer(ctx)?;

                if value_t.clone().unwrap().get_name() != t.clone().unwrap().get_name() {
                    return Err(Error::new_type_error(
                        ctx.input.clone(),
                        self.value.token.clone(),
                        t.clone(),
                        value_t.clone(),
                    ));
                }

                self.t = t.clone();

                Ok(t)
            } else {
                let t = self.value.infer(ctx)?;

                self.t = t.clone();

                ctx.scopes.add(name.clone(), t.clone());

                Ok(t)
            }
        } else {
            self.name.infer(ctx)?;

            let t = self.value.infer(ctx)?;

            self.t = t.clone();

            Ok(t)
        }
    }
}

impl IrBuilder for Assignation {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        unsafe {
            let ptr = if !self.name.has_secondaries() {
                let name = self.name.get_identifier().unwrap();

                if let Some(val) = context.scopes.get(name.clone()) {
                    val
                } else {
                    if let Some(t) = &self.t {
                        if let Some(_) = context.classes.get(&t.get_name()) {
                            let val = self.value.build(context).unwrap();

                            context.scopes.add(name.clone(), val);

                            return Some(val);
                        }
                    }

                    let mut alloc_name = "alloc_".to_string() + &name.clone();

                    alloc_name.push('\0');

                    let alloc = LLVMBuildAlloca(
                        context.builder,
                        get_type(Box::new(self.t.clone().unwrap()), context),
                        alloc_name.as_ptr() as *const _,
                    );

                    context.scopes.add(name.clone(), alloc);

                    alloc
                }
            } else {
                let ptr = self.name.build_no_load(context).unwrap();

                ptr
            };

            let val = self.value.build(context).unwrap();

            LLVMBuildStore(context.builder, val, ptr);

            Some(val)
        }
    }
}
