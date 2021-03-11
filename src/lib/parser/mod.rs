mod lexer;
mod parser_impl;
mod parsing_context;
mod span;
mod token;

pub use lexer::*;
pub use parser_impl::*;
pub use parsing_context::*;
pub use span::*;
pub use token::*;

use crate::diagnostics::Diagnostic;

fn parse_generic<F, R>(ctx: &mut ParsingCtx, mut f: F) -> Result<(R, Vec<Token>), Diagnostic>
where
    F: FnMut(&mut Parser) -> Result<R, Diagnostic>,
{
    let tokens = Lexer::new(ctx).collect();

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

pub fn parse_root(ctx: &mut ParsingCtx) -> Result<(crate::ast::Root, Vec<Token>), Diagnostic> {
    info!("      -> Parsing Root");

    parse_generic(ctx, |p| p.run_root())
}

pub fn parse_mod(
    name: String,
    ctx: &mut ParsingCtx,
) -> Result<(crate::ast::Mod, Vec<Token>), Diagnostic> {
    info!("      -> Parsing Mod {}", name);

    ctx.resolve_and_add_file(name)?;

    parse_generic(ctx, |p| p.run_mod())
}
