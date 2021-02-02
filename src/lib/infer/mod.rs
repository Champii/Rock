mod annotate;
mod constraint;
mod state;

use self::annotate::AnnotateContext;
use self::constraint::ConstraintContext;
pub use self::state::*;

pub fn infer(root: &crate::hir::Root) {
    let mut infer_state = InferState::new();

    let mut annotate_ctx = AnnotateContext::new(infer_state);

    annotate_ctx.annotate(root);

    let mut constraint_ctx = ConstraintContext::new(annotate_ctx.get_state(), root);

    constraint_ctx.constraint(root);

    infer_state = constraint_ctx.get_state();

    infer_state.solve();

    println!("INFER {:#?}", infer_state);
}
