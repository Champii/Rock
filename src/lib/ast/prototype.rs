use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;
use crate::ast::PrimitiveType;
use crate::ast::Type;
use crate::ast::TypeInfer;

use crate::codegen::get_type;
use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use llvm_sys::core::LLVMInt32Type;
use llvm_sys::LLVMValue;

use crate::generator::Generate;
use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct Prototype {
    pub name: Option<String>,
    pub ret: Type,
    pub arguments: Vec<Type>,
}

impl Prototype {
    pub fn apply_name(&mut self) {
        let mut name = String::new();

        for ty in &self.arguments {
            name = name + &ty.get_name();
        }

        self.name = Some(self.name.clone().unwrap() + &name);
    }

    fn arguments_decl_type(ctx: &mut Parser) -> Result<Vec<Type>, Error> {
        let mut res = vec![];

        ctx.save();

        ctx.consume();

        loop {
            let t = try_or_restore!(Type::parse(ctx), ctx);

            res.push(t);

            if TokenType::Coma != ctx.cur_tok.t {
                break;
            }

            ctx.consume();
        }

        ctx.consume();

        ctx.save_pop();

        Ok(res)
    }
}

impl Parse for Prototype {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut name = None;
        let mut arguments = vec![];

        ctx.save();

        if let TokenType::Identifier(ident) = &ctx.cur_tok.t {
            name = Some(ident.clone());

            ctx.consume();
        }

        if TokenType::OpenParens == ctx.cur_tok.t
            || TokenType::Identifier(ctx.cur_tok.txt.clone()) == ctx.cur_tok.t
        {
            // manage error and restore here
            arguments = Prototype::arguments_decl_type(ctx)?;
            // arguments = ctx.arguments_decl_type()?;
        }

        let ret = if ctx.cur_tok.t == TokenType::DoubleSemiColon {
            expect_or_restore!(TokenType::DoubleSemiColon, ctx);

            try_or_restore_expect!(
                Type::parse(ctx),
                TokenType::Type(ctx.cur_tok.txt.clone()),
                ctx
            )
        } else {
            Type::Primitive(PrimitiveType::Void)
        };

        return Ok(Prototype {
            name,
            ret,
            arguments,
        });
    }
}

impl IrBuilder for Prototype {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        let name_orig = self.name.clone().unwrap_or("nop".to_string());

        let mut name = name_orig.clone();

        name.push('\0');

        let name = name.as_str();

        unsafe {
            let i32t = LLVMInt32Type();
            let mut argts = vec![];

            for arg in &self.arguments {
                let t = get_type(Box::new(arg.clone()), context);

                argts.push(t);
            }

            let function_type =
                llvm::core::LLVMFunctionType(i32t, argts.as_mut_ptr(), argts.len() as u32, 0);

            let function = llvm::core::LLVMAddFunction(
                context.module,
                name.as_ptr() as *const _,
                function_type,
            );

            context.scopes.add(name_orig.clone(), function);
            context.functions.add(name_orig, function);

            Some(function)
        }
    }
}

impl Generate for Prototype {
    fn generate(&mut self, _ctx: &mut Context) -> Result<(), Error> {
        Ok(())
    }
}

impl TypeInferer for Prototype {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("Prototype");

        ctx.externs
            .insert(self.name.clone().unwrap(), self.name.clone().unwrap());

        ctx.scopes.add(
            self.name.clone().unwrap(),
            Some(Type::Proto(Box::new(self.clone()))),
        );

        Ok(Some(Type::Primitive(PrimitiveType::Void)))
    }
}
