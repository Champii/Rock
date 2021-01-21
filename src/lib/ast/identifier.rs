use crate::Parser;
use crate::TokenType;
use crate::{token::TokenId, Error};

use crate::ast::ast_print::*;
use crate::ast::Parse;

use crate::parser::macros::*;

#[derive(Debug, Clone)]
pub struct Identifier {
    pub name: String,
    pub token: TokenId,
}

impl Parse for Identifier {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let token_id = ctx.cur_tok_id;

        let token = expect!(TokenType::Identifier(ctx.cur_tok().txt.clone()), ctx);

        Ok(Self {
            name: token.txt,
            token: token_id,
        })
    }
}

impl AstPrint for Identifier {
    fn print(&self, ctx: &mut AstPrintContext) {
        let indent_str = String::from("  ").repeat(ctx.indent());

        println!(
            "{}Identifier {:?}",
            indent_str,
            ctx.get_token(self.token).unwrap().t
        );
    }
}
