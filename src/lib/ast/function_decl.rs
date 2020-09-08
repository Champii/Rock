use crate::Error;
use crate::Parser;
use crate::Token;
use crate::TokenType;

use crate::ast::argument_decl::ArgumentsDecl;
use crate::ast::r#type::TypeInfer;
use crate::ast::ArgumentDecl;
use crate::ast::Body;
use crate::ast::Identifier;
use crate::ast::Parse;
use crate::ast::Type;

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
        let mut name = String::new();

        for ty in t {
            name = name + &ty.unwrap().get_name();
        }

        self.name = self.name.clone() + &name;
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
