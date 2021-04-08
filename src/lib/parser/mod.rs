mod lexer;
mod parser_impl;
mod parsing_context;
mod source_file;
mod span;
mod token;

pub use lexer::*;
pub use parser_impl::*;
pub use parsing_context::*;
pub use source_file::*;
pub use span::*;
pub use token::*;

use crate::ast::visit::*;
use crate::{ast::ast_print::AstPrintContext, diagnostics::Diagnostic};

fn parse_generic<F, R>(ctx: &mut ParsingCtx, mut f: F) -> Result<(R, Vec<Token>), Diagnostic>
where
    F: FnMut(&mut Parser) -> Result<R, Diagnostic>,
{
    let tokens = Lexer::new(ctx).collect();

    // Debug tokens
    if ctx.config.show_tokens {
        println!("TOKENS {:#?}", tokens);
    }

    ctx.return_if_error()?;

    let mut parser = Parser::new(tokens.clone(), ctx);

    let ast = match f(&mut parser) {
        Ok(ast) => ast,
        Err(e) => {
            ctx.return_if_error()?;

            return Err(e);
        }
    };

    Ok((ast, tokens))
}

pub fn parse_root(ctx: &mut ParsingCtx) -> Result<crate::ast::Root, Diagnostic> {
    info!("      -> Parsing Root");

    let (ast, tokens) = parse_generic(ctx, |p| p.run_root())?;

    // Debug ast
    if ctx.config.show_ast {
        AstPrintContext::new(tokens, ctx.get_current_file()).visit_root(&ast);
    }

    Ok(ast)
}

pub fn parse_mod(name: String, ctx: &mut ParsingCtx) -> Result<crate::ast::Mod, Diagnostic> {
    info!("      -> Parsing Mod {}", name);

    ctx.resolve_and_add_file(name)?;

    let (ast, tokens) = parse_generic(ctx, |p| p.run_mod())?;

    // Debug ast
    if ctx.config.show_ast {
        AstPrintContext::new(tokens.clone(), ctx.get_current_file()).visit_mod(&ast);
    }

    Ok(ast)
}
