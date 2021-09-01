mod annotate;
mod constraint;
mod state;

use crate::Config;

use self::annotate::AnnotateContext;
use self::constraint::ConstraintContext;
pub use self::state::*;

pub fn infer(root: &mut crate::hir::Root, config: &Config) {
    let mut infer_state = InferState::new();

    let mut annotate_ctx = AnnotateContext::new(infer_state, root.trait_methods.clone());

    annotate_ctx.annotate(root);

    let mut constraint_ctx = ConstraintContext::new(annotate_ctx.get_state(), root);

    constraint_ctx.constraint(root);

    infer_state = constraint_ctx.get_state();

    println!("STATE {:#?}", infer_state);
    infer_state.solve();
    println!("STATE 2 {:#?}", infer_state);
    // println!("LOL {:#?}", root);

    // Here add trait solving

    root.node_types = infer_state.get_node_types();

    root.types = infer_state.get_types();

    if config.show_hir {
        println!("{:#?}", root);
    }
}
