mod annotate;
mod constraint;
mod mangle;
mod state;

use crate::{
    diagnostics::Diagnostic, hir::visit_mut::*, infer::mangle::*, parser::ParsingCtx, Config,
};

use self::annotate::AnnotateContext;
use self::constraint::ConstraintContext;
pub use self::state::*;

pub fn infer(
    root: &mut crate::hir::Root,
    parsing_ctx: &mut ParsingCtx,
    config: &Config,
) -> Result<(), Diagnostic> {
    let infer_state = InferState::new(root.clone()); // FIXME: Don't clone the whole hir !!!

    // infer_state.diagnostics = diagnostics;

    let mut annotate_ctx = AnnotateContext::new(infer_state, root.trait_methods.clone());

    annotate_ctx.annotate(root);

    let mut constraint_ctx = ConstraintContext::new(annotate_ctx.get_state(), root);

    constraint_ctx.constraint(root);

    let (mut infer_state, new_resolutions) = constraint_ctx.get_state();

    // // FIXME: don't
    for (k, v) in new_resolutions {
        root.resolutions.insert(k.clone(), v.clone());
    }

    if let Err(diag) = infer_state.solve() {
        parsing_ctx.diagnostics.push_error(diag.clone());
    }

    if config.show_state {
        println!("ROOT {:#?}", root);
        println!("STATE {:#?}", infer_state);
    }

    let mut mangle_ctx = MangleContext {
        trait_call_to_mangle: infer_state.trait_call_to_mangle.clone(),
    };

    mangle_ctx.visit_root(root);

    root.trait_call_to_mangle = infer_state.trait_call_to_mangle.clone();

    root.node_types = infer_state.get_node_types();

    let (types, diags) = infer_state.get_types();

    // we dont exit on unresolved types .. yet
    for diag in diags {
        parsing_ctx.diagnostics.push_warning(diag);
    }

    parsing_ctx.return_if_error()?;

    root.types = types;

    if config.show_hir {
        println!("{:#?}", root);
    }

    Ok(())
}
