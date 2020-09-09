use crate::Error;
use crate::Parser;
use crate::Token;
use crate::TokenType;

use crate::ast::Attribute;
use crate::ast::Expression;
use crate::ast::FunctionDecl;
use crate::ast::Identifier;
use crate::ast::Parse;
use crate::ast::Type;
use crate::ast::TypeInfer;

use crate::codegen::get_type;
use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use llvm_sys::core::LLVMStructType;
use llvm_sys::LLVMValue;

use crate::generator::Generate;
use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct Class {
    pub name: Identifier,
    pub attributes: Vec<Attribute>,
    pub class_attributes: Vec<Attribute>, // [(name, type, default)]
    pub methods: Vec<FunctionDecl>,
    pub class_methods: Vec<FunctionDecl>,
    pub token: Token,
}

impl Class {
    pub fn get_attribute(&self, name: String) -> Option<(Attribute, usize)> {
        let mut i: usize = 0;

        for attr in self.attributes.clone() {
            if name == attr.name {
                return Some((attr.clone(), i));
            }

            i += 1;
        }

        None
    }

    pub fn get_method(&self, name: String) -> Option<FunctionDecl> {
        for method in self.methods.clone() {
            if name == method.name {
                return Some(method.clone());
            }
        }

        None
    }
}

impl Parse for Class {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let tok_name = expect!(TokenType::Type(ctx.cur_tok.txt.clone()), ctx);

        ctx.save();

        expect!(TokenType::EOL, ctx);

        let mut attributes = vec![];
        let class_attributes = vec![];
        let mut methods = vec![];
        let class_methods = vec![];

        ctx.block_indent += 1;

        loop {
            if let TokenType::Indent(nb) = ctx.cur_tok.t {
                if nb != ctx.block_indent {
                    ctx.block_indent -= 1;

                    ctx.save_pop();

                    return Ok(Class {
                        name: tok_name.clone().txt,
                        attributes,
                        class_attributes,
                        methods,
                        class_methods,
                        token: tok_name,
                    });
                }

                ctx.consume();

                if let Ok(f) = FunctionDecl::parse(ctx) {
                    let mut f = f;

                    f.name = tok_name.txt.clone() + "_" + &f.name;

                    f.class_name = Some(tok_name.txt.clone());

                    f.add_this_arg();

                    methods.push(f);
                } else {
                    if let TokenType::Identifier(id) = ctx.cur_tok.t.clone() {
                        let token = ctx.cur_tok.clone();

                        ctx.consume();

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

                        let default = if ctx.cur_tok.t == TokenType::SemiColon {
                            expect_or_restore!(TokenType::SemiColon, ctx);

                            Some(try_or_restore!(Expression::parse(ctx), ctx))
                        } else {
                            None
                        };

                        attributes.push(Attribute {
                            name: id,
                            t: ret,
                            default,
                            token,
                        });

                        expect!(TokenType::EOL, ctx);

                        // if let Ok(fun) = ctx.function_decl() {
                        //     methods.push(fun);
                        // }
                    }
                }

            // property
            } else {
                break;
            }
        }

        ctx.block_indent -= 1;

        ctx.save_pop();

        let class = Class {
            name: tok_name.clone().txt,
            attributes,
            class_attributes,
            methods,
            class_methods,
            token: tok_name,
        };

        ctx.ctx.classes.insert(class.name.clone(), class.clone());

        Ok(class)
    }
}

impl TypeInferer for Class {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("Class ({:?})", self.token);

        // let t = TypeInfer::Type(Some(Type::Name(self.name.clone())));
        let t = Some(Type::Class(self.name.clone()));

        debug!("Creating type {:?} for class {}", t, self.name);

        ctx.scopes.add(self.name.clone(), t.clone());

        for attr in &mut self.attributes {
            attr.infer(ctx)?;
        }

        ctx.classes.insert(self.name.clone(), self.clone());

        for method in &mut self.methods {
            method.infer(ctx)?;
        }

        ctx.classes.insert(self.name.clone(), self.clone());

        Ok(t)
    }
}

impl Generate for Class {
    fn generate(&mut self, _ctx: &mut Context) -> Result<(), Error> {
        Ok(())
    }
}

impl IrBuilder for Class {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        let mut attrs_types = vec![];

        for attr in self.attributes.clone() {
            attrs_types.push(get_type(Box::new(attr.t.clone().unwrap()), context));
        }

        unsafe {
            let t = LLVMStructType(attrs_types.as_ptr() as *mut _, attrs_types.len() as u32, 0);

            context.classes.insert(self.name.clone(), (t, self.clone()));
        }

        None
    }
}
