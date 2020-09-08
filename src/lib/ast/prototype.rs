use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;
use crate::ast::PrimitiveType;
use crate::ast::Type;

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
