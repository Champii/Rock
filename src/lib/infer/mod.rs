mod constraint;
mod mangle;
mod monomorphizer;
mod state;

pub use self::state::*;

use crate::{diagnostics::Diagnostic, parser::ParsingCtx, Config};

pub fn infer(
    root: &mut crate::hir::Root,
    parsing_ctx: &mut ParsingCtx,
    config: &Config,
) -> Result<crate::hir::Root, Diagnostic> {
    let diags = constraint::solve(root);

    parsing_ctx.diagnostics.append(diags);

    let mut new_root = monomorphizer::monomophize(root);

    mangle::mangle(&mut new_root);

    if config.show_hir {
        super::hir::hir_printer::print(&new_root);
    }

    parsing_ctx.return_if_error()?;

    Ok(new_root)
}
