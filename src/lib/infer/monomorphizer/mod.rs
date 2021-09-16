use std::collections::{BTreeMap, HashMap};

use crate::{ast::resolve::ResolutionMap, hir::Root};

use self::monomorphizer::Monomorphizer;

mod monomorphizer;

pub fn monomophize(root: &mut Root) -> Root {
    Monomorphizer {
        root,
        trans_resolutions: ResolutionMap::default(),
        new_resolutions: ResolutionMap::default(),
        old_ordered_resolutions: HashMap::new(),
        body_arguments: BTreeMap::new(),
        generated_fn_hir_id: HashMap::new(),
    }
    .run()
}
