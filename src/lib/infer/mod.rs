mod annotate;
mod constraint;
mod mangle;
mod state;

use crate::{hir::visit_mut::*, infer::mangle::*, Config};

use self::annotate::AnnotateContext;
use self::constraint::ConstraintContext;
pub use self::state::*;

pub fn infer(root: &mut crate::hir::Root, config: &Config) {
    let infer_state = InferState::new();

    let mut annotate_ctx = AnnotateContext::new(infer_state, root.trait_methods.clone());

    annotate_ctx.annotate(root);

    let mut constraint_ctx = ConstraintContext::new(annotate_ctx.get_state(), root);

    constraint_ctx.constraint(root);

    let (mut infer_state, new_resolutions) = constraint_ctx.get_state();

    for (k, v) in new_resolutions {
        root.resolutions.insert(k.clone(), v.clone());
    }

    infer_state.solve();
    if config.show_state {
        println!("STATE {:#?}", infer_state);
        println!("ROOT {:#?}", root);
    }

    let mut mangle_ctx = MangleContext {
        trait_call_to_mangle: infer_state.trait_call_to_mangle.clone(),
    };

    mangle_ctx.visit_root(root);

    root.trait_call_to_mangle = infer_state.trait_call_to_mangle.clone();

    // println!("STATE 2 {:#?}", infer_state);

    // println!("LOL {:#?}", root);

    // Here add trait solving

    root.node_types = infer_state.get_node_types();

    root.types = infer_state.get_types();

    if config.show_hir {
        println!("{:#?}", root);
    }
}
