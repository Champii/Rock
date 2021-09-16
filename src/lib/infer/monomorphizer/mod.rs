use std::collections::{BTreeMap, HashMap};

use crate::{ast::resolve::ResolutionMap, hir::Root};

use self::monomorphizer::Monomorphizer;

mod call_collector;
mod call_solver;
mod monomorphizer;
mod proto_collector;

pub fn monomophize(root: &mut Root) -> Root {
    let protos = proto_collector::collect_prototypes(root);
    let calls = call_collector::collect_calls(root);

    let bindings = call_solver::solve_calls(protos, calls, root);

    Monomorphizer {
        root,
        bindings,
        trans_resolutions: ResolutionMap::default(),
        new_resolutions: ResolutionMap::default(),
        old_ordered_resolutions: HashMap::new(),
        body_arguments: BTreeMap::new(),
        generated_fn_hir_id: HashMap::new(),
    }
    .run()
}
