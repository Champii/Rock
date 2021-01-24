use crate::parser::macros::*;
use crate::Error;
use crate::Parser;
use crate::{infer::*, token::TokenType};

use crate::ast::Expression;
use crate::ast::Identifier;
use crate::ast::Identity;
use crate::ast::Literal;
use crate::ast::Parse;

#[derive(Debug, Clone)]
pub enum OperandKind {
    Literal(Literal),
    Identifier(Identifier),
    // ClassInstance(ClassInstance),
    // Array(Array),
    Expression(Box<Expression>), // parenthesis
}

visitable_constraint_enum!(
    OperandKind,
    ConstraintGen,
    constrain,
    InferBuilder,
    [Literal(x), Identifier(x), Expression(x)]
);

#[derive(Debug, Clone)]
pub struct Operand {
    pub kind: OperandKind,
    pub identity: Identity,
}

visitable_constraint_class!(Operand, ConstraintGen, constrain, InferBuilder, [kind]);

impl Operand {
    fn parens_expr(ctx: &mut Parser) -> Result<Expression, Error> {
        if ctx.cur_tok().t != TokenType::OpenParens {
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
        let mut token = ctx.cur_tok_id;

        let kind = if let Ok(lit) = Literal::parse(ctx) {
            OperandKind::Literal(lit)
        } else if let Ok(ident) = Identifier::parse(ctx) {
            OperandKind::Identifier(ident)
        // } else if let Ok(c) = ClassInstance::parse(ctx) {
        //     OperandKind::ClassInstance(c)
        // } else if let Ok(array) = Array::parse(ctx) {
        //     OperandKind::Array(array)
        } else if let Ok(expr) = Self::parens_expr(ctx) {
            token = expr.identity.token_id;

            OperandKind::Expression(Box::new(expr))
        } else {
            self::error!("Expected operand".to_string(), ctx);
        };

        return Ok(Operand {
            kind,
            identity: Identity::new(token),
        });
    }
}
