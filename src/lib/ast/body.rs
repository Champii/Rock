use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;
use crate::ast::Statement;

#[derive(Debug, Clone)]
pub struct Body {
    pub stmts: Vec<Statement>,
}

impl Parse for Body {
    fn parse(ctx: &mut Parser) -> Result<Self, Error> {
        let mut stmts = vec![];
        let mut is_multi = false;

        if ctx.cur_tok.t == TokenType::EOL {
            is_multi = true;

            ctx.block_indent += 1;

            ctx.consume();
        }

        if is_multi {
            if let TokenType::Indent(nb) = ctx.cur_tok.t {
                if nb != ctx.block_indent {
                    return Ok(Body { stmts });
                }

                ctx.consume();
            } else {
                return Ok(Body { stmts });
            }
        }

        loop {
            if ctx.cur_tok.t != TokenType::EOL && ctx.cur_tok.t != TokenType::EOF {
                match Statement::parse(ctx) {
                    Ok(stmt) => stmts.push(stmt),
                    Err(_) => break,
                };
            }

            if ctx.cur_tok.t != TokenType::EOF && is_multi {
                while ctx.cur_tok.t == TokenType::EOL {
                    ctx.consume();
                }
            }

            if is_multi {
                if let TokenType::Indent(nb) = ctx.cur_tok.t {
                    if nb != ctx.block_indent {
                        break;
                    }

                    ctx.consume();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if is_multi {
            ctx.block_indent -= 1;
        }

        Ok(Body { stmts })
    }
}
