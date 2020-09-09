use std::collections::HashMap;

use crate::Error;
use crate::Parser;
use crate::Token;
use crate::TokenType;

use crate::ast::Attribute;
use crate::ast::Class;
use crate::ast::Expression;
use crate::ast::Parse;
use crate::ast::Type;
use crate::ast::TypeInfer;

use crate::context::Context;
use crate::type_checker::TypeInferer;

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct ClassInstance {
    pub name: String,
    pub class: Class,
    pub attributes: HashMap<String, Attribute>,
    pub token: Token,
}

impl ClassInstance {
    pub fn get_attribute(&self, name: String) -> Option<(Attribute, usize)> {
        let mut i: usize = 0;

        for (_, attr) in self.attributes.clone() {
            if name == attr.name {
                return Some((attr.clone(), i));
            }

            i += 1;
        }

        None
    }
}

impl Parse for ClassInstance {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        ctx.save();

        let token = ctx.cur_tok.clone();

        let name = try_or_restore!(Type::parse(ctx), ctx).get_name();

        let mut attributes = HashMap::new();

        if let TokenType::OpenBrace = ctx.cur_tok.t {
            ctx.consume();
            ctx.consume(); // close

            ctx.save_pop();

            return Ok(ClassInstance {
                attributes,
                class: ctx.ctx.classes.get(&name.clone()).unwrap().clone(),
                name,
                token,
            });
        }

        expect_or_restore!(TokenType::EOL, ctx);

        ctx.block_indent += 1;

        let mut is_first = true;

        loop {
            ctx.save();

            if !is_first {
                expect_or_restore!(TokenType::EOL, ctx);
            }

            if let TokenType::Indent(nb) = ctx.cur_tok.t {
                if nb != ctx.block_indent {
                    ctx.restore();

                    // if is_first {
                    //     expect_or_restore!(TokenType::EOL, ctx);
                    // }

                    break;
                }

                ctx.consume();
            } else {
                ctx.restore();

                break;
            }

            if let TokenType::Identifier(id) = ctx.cur_tok.t.clone() {
                ctx.consume();

                expect!(TokenType::SemiColon, ctx);

                if let Ok(expr) = Expression::parse(ctx) {
                    attributes.insert(
                        id.clone(),
                        Attribute {
                            name: id,
                            t: None,
                            default: Some(expr),
                            token: ctx.cur_tok.clone(),
                        },
                    );
                } else {
                    ctx.restore();

                    break;
                }
            } else {
                ctx.restore();

                break;
            }

            ctx.save_pop();

            is_first = false;
        }

        // let

        ctx.save_pop();

        ctx.block_indent -= 1;

        Ok(ClassInstance {
            attributes,
            class: ctx.ctx.classes.get(&name.clone()).unwrap().clone(),
            name,
            token,
        })
    }
}

impl TypeInferer for ClassInstance {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("Expression ({:?})", self.token);

        for class_attr in &self.class.attributes {
            if let Some(attr) = self.attributes.get(&class_attr.name) {
                let mut attr = attr.clone();
                let attr_t = attr.infer(ctx)?;

                if class_attr.t.is_some() && class_attr.t != attr_t {
                    return Err(Error::new_type_error(
                        ctx.input.clone(),
                        attr.token.clone(),
                        class_attr.t.clone(),
                        attr_t.clone(),
                    ));
                }
            } else if class_attr.default.is_none() {
                return Err(Error::new_undefined_error(
                    ctx.input.clone(),
                    self.class.name.clone() + "::" + &class_attr.name.clone(),
                ));
            }
        }

        Ok(ctx.scopes.get(self.name.clone()).unwrap())
    }
}
