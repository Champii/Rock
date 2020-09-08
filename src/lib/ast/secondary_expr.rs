use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Expression;
use crate::ast::Parse;
use crate::ast::Selector;
use crate::ast::{Argument, Arguments};

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub enum SecondaryExpr {
    Selector(Selector), // . Identifier  // u8 is the attribute index in struct // option<Type> is the class type if needed // RealFullName
    Arguments(Vec<Argument>), // (Expr, Expr, ...)
    Index(Box<Expression>), // [Expr]
}

impl SecondaryExpr {
    fn index(ctx: &mut Parser) -> Result<Box<Expression>, Error> {
        ctx.save();

        expect_or_restore!(TokenType::OpenArray, ctx);

        let expr = try_or_restore!(Expression::parse(ctx), ctx);

        expect_or_restore!(TokenType::CloseArray, ctx);

        ctx.save_pop();

        return Ok(Box::new(expr));
    }
}

impl Parse for SecondaryExpr {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        if let Ok(idx) = Self::index(ctx) {
            return Ok(SecondaryExpr::Index(idx));
        }

        if let Ok(sel) = Selector::parse(ctx) {
            return Ok(SecondaryExpr::Selector(sel));
        }

        if let Ok(args) = Arguments::parse(ctx) {
            return Ok(SecondaryExpr::Arguments(args));
        }

        self::error!("Expected secondary".to_string(), ctx);
    }
}
