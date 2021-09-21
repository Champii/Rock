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
    let (tmp_resolutions, diags) = constraint::solve(root);
    // super::hir::hir_printer::print(&*root);

    parsing_ctx.diagnostics.append(diags);

    parsing_ctx.return_if_error()?;

    let mut new_root = monomorphizer::monomophize(root, tmp_resolutions);

    mangle::mangle(&mut new_root);

    if config.show_hir {
        super::hir::hir_printer::print(&new_root);
    }

    parsing_ctx.return_if_error()?;

    Ok(new_root)
}
