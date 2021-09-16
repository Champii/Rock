// mod annotate;
// mod constraint;
mod constraint2;
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
    super::hir::hir_printer::print(root);

    // let (mut new_root, diags) = constraint::solve(new_root);
    let diags = constraint2::solve(root);

    let mut new_root = monomorphizer::monomophize(root);
    println!("ROOT {:#?}", new_root);
    super::hir::hir_printer::print(&new_root);

    mangle::mangle(&mut new_root);

    parsing_ctx.diagnostics.append(diags);

    if config.show_hir {}

    parsing_ctx.return_if_error()?;

    Ok(new_root)
}
