use std::collections::HashMap;

use crate::Error;
use crate::Parser;
use crate::Token;
use crate::TokenType;

use crate::ast::Argument;
use crate::ast::Expression;
use crate::ast::ExpressionKind;
use crate::ast::Parse;
use crate::ast::PrimitiveType;
use crate::ast::SecondaryExpr;
use crate::ast::Type;
use crate::ast::TypeInfer;
use crate::ast::UnaryExpr;
use crate::ast::{Operand, OperandKind};

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use llvm_sys::core::LLVMBuildLoad;
use llvm_sys::LLVMValue;

use crate::generator::Generate;

#[derive(Debug, Clone)]
pub enum PrimaryExpr {
    PrimaryExpr(Operand, Vec<SecondaryExpr>),
}

impl PrimaryExpr {
    pub fn has_secondaries(&self) -> bool {
        match self {
            PrimaryExpr::PrimaryExpr(_, vec) => vec.len() > 0,
        }
    }

    pub fn get_identifier(&self) -> Option<String> {
        match self {
            PrimaryExpr::PrimaryExpr(op, _) => {
                if let OperandKind::Identifier(ident) = &op.kind {
                    Some(ident.clone())
                } else {
                    None
                }
            }
        }
    }

    pub fn build_no_load(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        match self {
            PrimaryExpr::PrimaryExpr(operand, vec) => {
                let mut op = operand.build(context);

                if vec.len() == 0 {
                    return op;
                }

                let mut last = vec.first().unwrap().clone();
                let mut is_first = true;

                for second in vec {
                    if !is_first {
                        if let SecondaryExpr::Selector(_) = second {
                            op = if let SecondaryExpr::Selector(_) = last {
                                unsafe {
                                    Some(LLVMBuildLoad(
                                        context.builder,
                                        op.clone().unwrap(),
                                        b"\0".as_ptr() as *const _,
                                    ))
                                }
                            } else {
                                op
                            };
                        }
                    }

                    op = second.build_with(context, op.clone().unwrap());

                    last = second.clone();

                    is_first = false;
                }

                op
            }
        }
    }
}

impl Generate for PrimaryExpr {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        match self {
            PrimaryExpr::PrimaryExpr(ref mut operand, vec) => {
                let mut s = String::new();
                let mut res = if let OperandKind::Identifier(ref mut id) = &mut operand.kind {
                    id
                } else {
                    &mut s
                };

                let mut last_method = None;
                let mut already_mangled = false;

                for second in vec {
                    match second {
                        SecondaryExpr::Selector(sel) => {
                            last_method = sel.class_type.clone();

                            if sel.full_name != sel.name {
                                already_mangled = true;
                            }

                            res = &mut sel.full_name;
                        }
                        SecondaryExpr::Arguments(args) => {
                            let mut name = res.clone();

                            if already_mangled {
                                continue;
                            }

                            if let Some(classname) = last_method.clone() {
                                name = classname.get_name() + "_" + &name;
                            }

                            let mut ctx_save = ctx.clone();

                            for arg in args {
                                if arg.t.is_none() {
                                    let t = arg.infer(&mut ctx_save).unwrap();
                                    arg.t = t.clone();
                                }

                                arg.generate(&mut ctx_save)?;

                                name = name.to_owned() + &arg.clone().t.unwrap().get_name();
                            }

                            *ctx = ctx_save;

                            if ctx.externs.get(res).is_none() && !already_mangled {
                                *res = name;
                            }
                        }
                        _ => (),
                    };
                }

                Ok(())
            }
        }
    }
}

impl Parse for PrimaryExpr {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let operand = Operand::parse(ctx)?;

        let mut secondarys = vec![];

        if ctx.cur_tok.t == TokenType::Operator(ctx.cur_tok.txt.clone())
            || ctx.cur_tok.t == TokenType::Equal
        {
            return Ok(PrimaryExpr::PrimaryExpr(operand, secondarys));
        }

        while let Ok(second) = SecondaryExpr::parse(ctx) {
            secondarys.push(second);

            if ctx.cur_tok.t == TokenType::Operator(ctx.cur_tok.txt.clone())
                || ctx.cur_tok.t == TokenType::Equal
            {
                break;
            }
        }

        Ok(PrimaryExpr::PrimaryExpr(operand, secondarys))
    }
}

impl TypeInferer for PrimaryExpr {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        match self {
            PrimaryExpr::PrimaryExpr(operand, ref mut vec) => {
                trace!("PrimaryExpr");

                ctx.cur_type = operand.infer(ctx)?;

                if vec.len() == 0 {
                    return Ok(ctx.cur_type.clone());
                }

                let mut prec = vec![];

                for second in vec {
                    match second {
                        SecondaryExpr::Arguments(ref mut args) => {
                            let mut args_types = vec![];

                            if let Some(Type::FuncType(f)) = &ctx.cur_type {
                                let mut name = f.name.clone();
                                let ret = f.ret.clone();

                                if args.len() < f.arguments.len() {
                                    if let Some(classname) = &f.class_name {
                                        let this = Argument {
                                            token: Token::default(),
                                            t: Some(Type::Class(classname.clone())),
                                            arg: Expression {
                                                kind: ExpressionKind::UnaryExpr(
                                                    UnaryExpr::PrimaryExpr(
                                                        PrimaryExpr::PrimaryExpr(
                                                            operand.clone(),
                                                            prec.clone(),
                                                        ),
                                                    ),
                                                ),
                                                t: Some(Type::Class(classname.clone())),
                                                token: Token::default(),
                                            },
                                        };

                                        args.insert(0, this);
                                    }
                                }

                                let orig_name = name.clone();

                                for arg in args {
                                    let t = arg.infer(ctx)?;

                                    arg.t = t.clone();

                                    args_types.push(t.clone());

                                    name = name + &t.unwrap().get_name();
                                }

                                ctx.cur_type = ret;

                                ctx.calls
                                    .entry(orig_name.clone())
                                    .or_insert(HashMap::new())
                                    .insert(name, args_types);
                            } else if let Some(Type::Proto(proto)) = &ctx.cur_type {
                                ctx.calls
                                    .entry(proto.name.clone().unwrap())
                                    .or_insert(HashMap::new())
                                    .insert(proto.name.clone().unwrap(), args_types);

                                ctx.cur_type = Some(proto.ret.clone());
                            } else {
                                println!("AST {:?}", self);
                                panic!("WOUAT ?!");
                            }
                        }
                        // }
                        SecondaryExpr::Index(_args) => {
                            trace!("Index");

                            if let Some(t) = ctx.cur_type.clone() {
                                let t = t.clone();

                                if let Type::Primitive(PrimitiveType::Array(a, _n)) = t.clone() {
                                    ctx.cur_type = Some(*a);
                                } else if let Type::Primitive(PrimitiveType::String(_n)) = t.clone()
                                {
                                    ctx.cur_type = Some(Type::Primitive(PrimitiveType::Int8));
                                } else {
                                    return Err(Error::new_not_indexable_error(t.get_name()));
                                }

                                // TODO
                                // if let Type::Primitive(_p) = t {
                                //     ctx.cur_type =
                                //         Some(Type::Primitive(PrimitiveType::Int8));
                                // }
                            }
                        }

                        SecondaryExpr::Selector(ref mut sel) => {
                            trace!("Selector ({:?}), class: {:?}", sel.name, sel.class_type);

                            if let Some(t) = ctx.cur_type.clone() {
                                let classname = t.get_name();

                                let class = ctx.classes.get(&classname);

                                if class.is_none() {
                                    return Err(Error::new_undefined_type(
                                        ctx.input.clone(),
                                        classname,
                                    ));
                                    // panic!("Unknown class {}", classname);
                                }

                                let class = class.unwrap();

                                let method_name = class.name.clone() + "_" + &sel.name.clone();

                                let f = class.get_method(method_name.clone());

                                if let Some(f) = f {
                                    let scope_f = ctx.scopes.get(f.name.clone()).clone().unwrap();

                                    ctx.cur_type = Some(Type::FuncType(Box::new(f)));

                                    if let Some(Type::FuncType(_)) = scope_f.clone() {
                                        ctx.cur_type = scope_f.clone();
                                    }

                                    sel.class_type = Some(t.clone()); // classname

                                    continue;
                                }

                                let attr = class.get_attribute(sel.name.clone());

                                if let None = attr {
                                    panic!("Unknown property {}", sel.name);
                                }

                                let (attr, i) = attr.unwrap();

                                sel.class_offset = i as u8; // attribute index

                                sel.class_type = Some(t.clone()); // classname

                                ctx.cur_type = attr.t.clone();
                            }
                        }
                    };

                    prec.push(second.clone());
                }

                Ok(ctx.cur_type.clone())
            }
        }
    }
}

impl IrBuilder for PrimaryExpr {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        let res = self.build_no_load(context);

        match self {
            PrimaryExpr::PrimaryExpr(_, vec) => {
                if vec.len() == 0 {
                    return res;
                }

                let last_second = vec.last().unwrap();

                if let SecondaryExpr::Arguments(_) = last_second {
                    return res;
                }

                unsafe {
                    let op = LLVMBuildLoad(
                        context.builder,
                        res.clone().unwrap(),
                        b"\0".as_ptr() as *const _,
                    );

                    Some(op)
                }
            }
        }
    }
}
