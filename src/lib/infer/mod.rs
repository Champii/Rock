mod annotate;
mod call_collector;
mod call_solver;
mod constraint;
mod mangle;
mod monomorphizer;
mod proto_collector;
mod state;

use crate::{
    diagnostics::Diagnostic,
    hir::{hir_printer::*, visit_mut::*, Arena},
    infer::mangle::*,
    parser::ParsingCtx,
    Config,
};

use self::annotate::AnnotateContext;
use self::constraint::ConstraintContext;
pub use self::state::*;

pub fn infer(
    root: &mut crate::hir::Root,
    parsing_ctx: &mut ParsingCtx,
    config: &Config,
) -> Result<crate::hir::Root, Diagnostic> {
    super::hir::hir_printer::print(root);

    let mut new_root = monomorphizer::monomophize(root);

    let (mut new_root, diags) = constraint::solve(new_root);

    mangle::mangle(&mut new_root);

    parsing_ctx.diagnostics.append(diags);

    if config.show_hir {
        super::hir::hir_printer::print(&new_root);
    }

    parsing_ctx.return_if_error()?;

    Ok(new_root)
}
