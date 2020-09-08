use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Array;
use crate::ast::ClassInstance;
use crate::ast::Expression;
use crate::ast::Identifier;
use crate::ast::Literal;
use crate::ast::Parse;
use crate::ast::TypeInfer;

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
