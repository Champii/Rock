use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::ast_print::*;
use crate::ast::Parse;

use crate::error;

#[derive(Debug, Clone)]
pub enum Operator {
    Add,
    Sub,
    Sum,
    Div,
    Mod,

    Less,
    LessOrEqual,
    More,
    MoreOrEqual,

    EqualEqual,
    DashEqual,
}

impl AstPrint for Operator {
    fn print(&self, ctx: &mut AstPrintContext) {
        let indent = String::from("  ").repeat(ctx.indent());

        println!("{}{:?}", indent, self);
    }
}

impl Parse for Operator {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let op = match ctx.cur_tok().t {
            TokenType::Operator(op) => op,
            _ => error!("Expected operator".to_string(), ctx),
        };

        let op = match op.as_ref() {
            "+" => Operator::Add,
            "==" => Operator::EqualEqual,
            "!=" => Operator::DashEqual,
            "<" => Operator::Less,
            "<=" => Operator::LessOrEqual,
            ">" => Operator::More,
            ">=" => Operator::MoreOrEqual,
            _ => Operator::Add,
        };

        ctx.consume();

        Ok(op)
    }
}
