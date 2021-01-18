use crate::Error;
use crate::Parser;
use crate::TokenType;
use llvm_sys::core::LLVMBuildAlloca;
use llvm_sys::core::LLVMBuildGEP;
use llvm_sys::core::LLVMBuildLoad;
use llvm_sys::core::LLVMBuildStore;
use llvm_sys::core::LLVMConstInt;
use llvm_sys::core::LLVMInt32Type;

use crate::ast::Array;
use crate::ast::ClassInstance;
use crate::ast::Expression;
use crate::ast::Identifier;
use crate::ast::Literal;
use crate::ast::Parse;
use crate::ast::Type;
use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use llvm_sys::LLVMValue;

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub enum OperandKind {
    Literal(Literal),
    Identifier(Identifier),
    ClassInstance(ClassInstance),
    Array(Array),
    Expression(Box<Expression>), // parenthesis
}

#[derive(Debug, Clone)]
pub struct Operand {
    pub kind: OperandKind,
    pub t: TypeInfer,
}

impl Operand {
    fn parens_expr(ctx: &mut Parser) -> Result<Expression, Error> {
        if ctx.cur_tok.t != TokenType::OpenParens {
            self::error!("No parens expr".to_string(), ctx);
        } else {
            ctx.save();

            expect_or_restore!(TokenType::OpenParens, ctx);

            let expr = try_or_restore!(Expression::parse(ctx), ctx);

            expect_or_restore!(TokenType::CloseParens, ctx);

            ctx.save_pop();

            Ok(expr)
        }
    }
}

impl Parse for Operand {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let kind = if let Ok(lit) = Literal::parse(ctx) {
            OperandKind::Literal(lit)
        } else if let Ok(ident) = Identifier::parse(ctx) {
            OperandKind::Identifier(ident)
        } else if let Ok(c) = ClassInstance::parse(ctx) {
            OperandKind::ClassInstance(c)
        } else if let Ok(array) = Array::parse(ctx) {
            OperandKind::Array(array)
        } else if let Ok(expr) = Self::parens_expr(ctx) {
            OperandKind::Expression(Box::new(expr))
        } else {
            self::error!("Expected operand".to_string(), ctx);
        };

        return Ok(Operand { kind, t: None });
    }
}

impl TypeInferer for Operand {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("Operand");

        let t = match &mut self.kind {
            OperandKind::Literal(lit) => lit.infer(ctx),
            OperandKind::Identifier(ident) => {
                let res = match ctx.scopes.get(ident.clone()) {
                    Some(res) => res,
                    None => {
                        return Err(Error::new_undefined_error(ctx.input.clone(), ident.clone()));
                    }
                };

                println!("ident {:?}", ident);
                if let None = res {
                    ctx.scopes.add(ident.clone(), ctx.cur_type.clone());

                    return Ok(ctx.cur_type.clone());
                } else {
                    Ok(res)
                }
            }
            OperandKind::ClassInstance(ci) => ci.infer(ctx),
            OperandKind::Array(arr) => arr.infer(ctx),
            OperandKind::Expression(expr) => expr.infer(ctx),
        };

        self.t = t?;

        Ok(self.t.clone())
    }
}

impl IrBuilder for Operand {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        match &self.kind {
            OperandKind::Literal(lit) => lit.build(context),
            OperandKind::Identifier(ident) => {
                if let Some(args) = context.arguments.get(ident.clone()) {
                    return Some(args);
                }

                if let Some(func) = context.functions.get(ident.clone()) {
                    return Some(func);
                }

                if let Some(ptr) = context.scopes.get(ident.clone()) {
                    unsafe {
                        let mut ident = ident.clone();

                        ident.push('\0');

                        if let Some(Type::Class(_)) = &self.t {
                            return Some(ptr);
                        }

                        Some(LLVMBuildLoad(
                            context.builder,
                            ptr,
                            ident.as_ptr() as *const _,
                        ))
                    }
                } else {
                    panic!("Unknown identifier {}", ident);
                }
            }
            OperandKind::ClassInstance(ci) => {
                if let Some(class_ty) = context.classes.get(&ci.name.clone()) {
                    unsafe {
                        let res = LLVMBuildAlloca(
                            context.builder,
                            class_ty.0.clone(),
                            "\0".as_ptr() as *const _,
                        );

                        let zero = LLVMConstInt(LLVMInt32Type(), 0, 0);

                        for attr in ci.class.attributes.clone() {
                            let class_attr = ci.class.get_attribute(attr.name.clone()).unwrap();

                            let (val, i) = match ci.get_attribute(attr.name.clone()) {
                                None => (class_attr.0.default.unwrap(), class_attr.1), // handle error here
                                Some((attr, _i)) => (attr.default.unwrap(), class_attr.1), // and here
                            };

                            let idx = LLVMConstInt(LLVMInt32Type(), i as u64, 0);
                            let mut indices = [zero, idx];

                            let ptr_elem = LLVMBuildGEP(
                                context.builder,
                                res,
                                indices.as_mut_ptr(),
                                2,
                                b"\0".as_ptr() as *const _,
                            );

                            let val_res = if val.is_identifier() {
                                let ident = val.get_identifier().unwrap();
                                let t = class_attr.0.t.clone().unwrap();
                                if let Type::Class(_) = t {
                                    context.scopes.get(ident).unwrap()
                                } else {
                                    val.build(context).unwrap()
                                }
                            } else {
                                val.build(context).unwrap()
                            };

                            LLVMBuildStore(context.builder, val_res, ptr_elem);
                        }

                        Some(res)
                    }
                } else {
                    panic!("Unknown class {}", ci.name);
                }
            }
            OperandKind::Array(arr) => arr.build(context),
            OperandKind::Expression(expr) => expr.build(context),
        }
    }
}
