use std::collections::{BTreeMap, HashMap};

use crate::{
    hir::{HirId, Root},
    resolver::ResolutionMap,
};

use self::monomorphizer::Monomorphizer;

mod monomorphizer;

pub fn monomophize(
    root: &mut Root,
    tmp_resolutions: BTreeMap<HirId, ResolutionMap<HirId>>,
) -> Root {
    Monomorphizer {
        root,
        trans_resolutions: ResolutionMap::default(),
        new_resolutions: ResolutionMap::default(),
        old_ordered_resolutions: HashMap::new(),
        body_arguments: BTreeMap::new(),
        generated_fn_hir_id: HashMap::new(),
        structs: HashMap::new(),
        tmp_resolutions,
    }
    .run()
}
