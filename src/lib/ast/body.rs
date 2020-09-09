use crate::Error;
use crate::Parser;
use crate::TokenType;

use crate::ast::Parse;
use crate::ast::Statement;
use crate::ast::TypeInfer;

use crate::codegen::IrBuilder;
use crate::codegen::IrContext;
use crate::context::Context;
use crate::type_checker::TypeInferer;

use crate::generator::Generate;
use llvm_sys::LLVMValue;

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

impl TypeInferer for Body {
    fn infer(&mut self, ctx: &mut Context) -> Result<TypeInfer, Error> {
        trace!("Body");

        let mut last = Err(Error::new_empty());

        for stmt in &mut self.stmts {
            last = Ok(stmt.infer(ctx)?);
        }

        last
    }
}

impl Generate for Body {
    fn generate(&mut self, ctx: &mut Context) -> Result<(), Error> {
        for stmt in &mut self.stmts {
            stmt.generate(ctx)?;
        }

        Ok(())
    }
}

impl IrBuilder for Body {
    fn build(&self, context: &mut IrContext) -> Option<*mut LLVMValue> {
        let mut last = None;

        for stmt in &self.stmts {
            last = stmt.build(context);
        }

        last
    }
}
