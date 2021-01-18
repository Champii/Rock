use llvm_sys::core::LLVMGetParam;
use llvm_sys::LLVMValue;

use crate::Error;
use crate::Parser;
use crate::Token;
use crate::TokenType;

use crate::ast::argument_decl::ArgumentsDecl;
use crate::ast::ArgumentDecl;
use crate::ast::Body;
use crate::ast::Identifier;
use crate::ast::Parse;
use crate::ast::PrimitiveType;
use crate::ast::Type;
use crate::ast::TypeInfer;

use crate::codegen::get_type;
use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;

use crate::type_checker::TypeInferer;

use crate::generator::Generate;
use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: Identifier,
    pub ret: Option<Type>,
    pub arguments: Vec<ArgumentDecl>,
    pub body: Body,
    pub class_name: Option<String>,
    pub token: Token,
}

impl FunctionDecl {
    pub fn add_this_arg(&mut self) {
        self.arguments.insert(
            0,
            ArgumentDecl {
                name: "this".to_string(),
                t: Some(Type::Class(self.class_name.clone().unwrap())),
                token: self.token.clone(),
            },
        )
    }

    pub fn is_solved(&self) -> bool {
        self.arguments.iter().all(|arg| arg.t.is_some()) && self.ret.is_some()
    }

    pub fn apply_name(&mut self, t: Vec<TypeInfer>) {
        let mut name = self.name.clone();

        for ty in t {
            name = name + &ty.unwrap().get_name();
        }

        self.name = name;
    }

    pub fn apply_name_self(&mut self) {
        let mut name = String::new();

        for arg in &self.arguments {
            name = name + &arg.t.clone().unwrap().get_name();
        }

        // if let Some(t) = self.ret.clone() {
        //     name = name + &t.get_name();
        // }

        self.name = self.name.clone() + &name;
    }

    pub fn apply_types(&mut self, ret: Option<Type>, t: Vec<TypeInfer>) {
        // self.apply_name(t.clone(), ret.clone());

        self.ret = ret;

        let mut i = 0;

        println!("APPLY_TYPE {:#?}", t);

        for arg in &mut self.arguments {
            if i >= t.len() {
                break;
            }

            arg.t = t[i].clone();

            i += 1;
        }
    }

    pub fn get_solved_name(&self) -> String {
        let orig_name = self.name.clone();

        // self.apply_name()

        orig_name
    }
}

impl Parse for FunctionDecl {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut arguments = vec![];

        let token = ctx.cur_tok.clone();

        ctx.save();

        let name = try_or_restore!(Identifier::parse(ctx), ctx);

        if let TokenType::SemiColon = ctx.cur_tok.t {
            ctx.consume();
        }

        if TokenType::OpenParens == ctx.cur_tok.t
            || TokenType::Identifier(ctx.cur_tok.txt.clone()) == ctx.cur_tok.t
        {
            // manage error and restore here
            arguments = ArgumentsDecl::parse(ctx)?;
        }

        let ret = if ctx.cur_tok.t == TokenType::DoubleSemiColon {
            expect_or_restore!(TokenType::DoubleSemiColon, ctx);

            Some(try_or_restore_expect!(
                Type::parse(ctx),
                TokenType::Type(ctx.cur_tok.txt.clone()),
                ctx
            ))
        } else {
            None
        };

        expect_or_restore!(TokenType::Arrow, ctx);

        let body = try_or_restore!(Body::parse(ctx), ctx);

        ctx.save_pop();

        Ok(FunctionDecl {
            name,
            ret,
            arguments,
            body,
            class_name: None,
            token,
        })
    }
}

impl TypeInferer for FunctionDecl {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("FunctionDecl ({:?})", self.token);

        ctx.scopes.push();

        let mut types = vec![];
        println!("FUNCDECL {:#?}", self.arguments);

        for arg in &mut self.arguments {
            let t = arg.infer(ctx)?;

            arg.t = t.clone();

            debug!("Infered type {:?} for argument {}", t, arg.name);

            types.push(arg.t.clone());
        }

        let last = self.body.infer(ctx)?;

        debug!("Infered type {:?} for return of func {}", last, self.name);

        if self.ret.is_none() {
            self.ret = last.clone();
        } else if self.ret != last {
            warn!(
                "Ignoring the return override ({:?} by {:?}) of func {}",
                self.ret, last, self.name
            )
        }

        let mut i = 0;

        for arg in &mut self.arguments {
            if arg.t != types[i] {
                debug!(
                    "Argument type has been overrided ({:?} by {:?}) for func {}",
                    types[i], arg.t, self.name
                );
            }

            arg.t = types[i].clone();

            i += 1;
        }

        ctx.scopes.pop();

        ctx.scopes.add(
            self.name.clone(),
            Some(Type::FuncType(Box::new(self.clone()))),
        );

        Ok(last)
    }
}

impl Generate for FunctionDecl {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        ctx.scopes.push();

        let res = self.body.generate(ctx);

        ctx.scopes.pop();

        res
    }
}

impl IrBuilder for FunctionDecl {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        let name_orig = self.name.clone();

        let mut name = name_orig.clone();

        name.push('\0');

        let name = name.as_str();

        if !self.is_solved() {
            panic!("CODEGEN: FuncDecl is not solved {}", self.name);
        }

        unsafe {
            let mut argts = vec![];

            for arg in &self.arguments {
                let t = get_type(Box::new(arg.t.clone().unwrap()), context);

                argts.push(t);
            }

            let function_type = llvm::core::LLVMFunctionType(
                get_type(Box::new(self.ret.clone().unwrap()), context),
                argts.as_mut_ptr(),
                argts.len() as u32,
                0,
            );

            let function = llvm::core::LLVMAddFunction(
                context.module,
                name.as_ptr() as *const _,
                function_type,
            );

            context.scopes.add(name_orig.clone(), function);
            context.functions.add(name_orig, function);

            context.scopes.push();
            context.functions.push();
            context.arguments.push();

            let mut count = 0;
            for arg in &self.arguments {
                context
                    .scopes
                    .add(arg.name.clone(), LLVMGetParam(function, count));
                context
                    .arguments
                    .add(arg.name.clone(), LLVMGetParam(function, count));

                count += 1;
            }

            let bb = llvm::core::LLVMAppendBasicBlockInContext(
                context.context,
                function,
                b"entry\0".as_ptr() as *const _,
            );

            llvm::core::LLVMPositionBuilderAtEnd(context.builder, bb);

            let res = self.body.build(context);

            match &self.ret {
                Some(Type::Primitive(p)) => match p {
                    PrimitiveType::Void => llvm::core::LLVMBuildRetVoid(context.builder),
                    _ => llvm::core::LLVMBuildRet(context.builder, res.unwrap()),
                },
                _ => llvm::core::LLVMBuildRet(context.builder, res.unwrap()),
            };

            context.scopes.pop();
            context.functions.pop();
            context.arguments.pop();

            return Some(function);
        }
    }
}
