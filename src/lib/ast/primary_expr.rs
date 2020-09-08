use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;
use crate::ast::SecondaryExpr;
use crate::ast::{Operand, OperandKind};

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
