use crate::ast::Class;
use crate::ast::FunctionDecl;
use crate::ast::Parse;
use crate::ast::Prototype;
use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::try_or_restore;

#[derive(Debug, Clone)]
pub enum TopLevel {
    Mod(String),
    Class(Class),
    Function(FunctionDecl),
    Prototype(Prototype),
}

impl Parse for TopLevel {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let res = match ctx.cur_tok.t {
            TokenType::ExternKeyword => {
                ctx.save();

                ctx.consume();

                let proto = try_or_restore!(Prototype::parse(ctx), ctx);

                ctx.save_pop();

                Ok(TopLevel::Prototype(proto))
            }
            TokenType::ClassKeyword => {
                ctx.save();

                ctx.consume();

                let class = try_or_restore!(Class::parse(ctx), ctx);

                ctx.save_pop();

                Ok(TopLevel::Class(class))
            }
            _ => Ok(TopLevel::Function(FunctionDecl::parse(ctx)?)),
        };

        while ctx.cur_tok.t == TokenType::EOL {
            ctx.consume();
        }

        res
    }
}
