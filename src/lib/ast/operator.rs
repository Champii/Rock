use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;

use crate::error;

// impl Annotate for Operator {
//     fn annotate(&self, _ctx: &mut InferBuilder) {
//         //
//     }
// }

impl Parse for Operator {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let op = match ctx.cur_tok().t {
            TokenType::Operator(op) => op,
            _ => error!("Expected operator".to_string(), ctx),
        };

        let op = match op.as_ref() {
            "+" => Operator::Add,
            "-" => Operator::Sub,
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

// impl ConstraintGen for Operator {
//     fn constrain(&self, _ctx: &mut InferBuilder) -> TypeId {
//         // ctx.get_type(self.identity.clone()).unwrap()
//         0
//     }
// }
