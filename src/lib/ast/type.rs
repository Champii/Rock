use std::fmt;

use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::FunctionDecl;
use crate::ast::Parse;
use crate::ast::PrimitiveType;
use crate::ast::Prototype;

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub enum Type {
    Primitive(PrimitiveType),
    Proto(Box<Prototype>),
    FuncType(Box<FunctionDecl>),
    Class(String),
    ForAll(String), // TODO
    Undefined(String),
}

impl PartialEq for Type {
    fn eq(&self, other: &Type) -> bool {
        self.get_name() == other.get_name()
    }
}

impl Type {
    pub fn get_name(&self) -> String {
        match self {
            Self::Primitive(p) => p.get_name(),
            Self::Proto(p) => p.name.clone().unwrap_or(String::new()),
            Self::FuncType(f) => f.name.clone(),
            Self::Class(c) => c.clone(),
            Self::ForAll(_) => String::new(),
            Self::Undefined(s) => s.clone(),
            // Type::Name(s) => s.clone(),
            // Type::Array(a, _) => "[]".to_string() + &a.get_name(),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}

pub type TypeInfer = Option<Type>;

impl Parse for Type {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if ctx.cur_tok.t == TokenType::ArrayType {
            ctx.save();

            ctx.consume();

            let t = try_or_restore!(Type::parse(ctx), ctx);

            ctx.save_pop();

            Ok(Type::Primitive(PrimitiveType::Array(Box::new(t), 0)))
        } else if let Some(t) = PrimitiveType::from_name(&ctx.cur_tok.txt) {
            ctx.consume();
            Ok(Type::Primitive(t))
        } else {
            Ok(Type::Class(
                expect!(TokenType::Type(ctx.cur_tok.txt.clone()), ctx).txt,
            ))
        }
    }
}
