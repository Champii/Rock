use std::collections::HashMap;

use crate::{ast_lowering::HirMap, hir::HirId, NodeId};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ResolutionMap<T>(HashMap<T, T>)
where
    T: Eq + Clone + std::hash::Hash + Default;

impl<T: Eq + Clone + std::hash::Hash + Default> ResolutionMap<T> {
    pub fn insert(&mut self, pointer_id: T, pointee_id: T) {
        if self.0.insert(pointer_id, pointee_id).is_some() {
            debug!("Overriding resolution");
        }
    }

    pub fn get(&self, pointer_id: &T) -> Option<T> {
        self.0.get(&pointer_id).cloned()
    }

    pub fn get_recur(&self, pointer_id: &T) -> Option<T> {
        self.get(pointer_id).and_then(|pointee_id| {
            if *pointer_id == pointee_id {
                warn!("Resolution loop");

                return Some(pointee_id);
            } else {
                self.get_recur(&pointee_id).or(Some(pointee_id))
            }
        })
    }

    pub fn get_map(&self) -> HashMap<T, T> {
        self.0.clone()
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn remove(&mut self, item: &T) {
        self.0.remove(item);
    }
}

impl ResolutionMap<NodeId> {
    pub fn lower_resolution_map(&self, hir_map: &HirMap) -> ResolutionMap<HirId> {
        ResolutionMap(
            self.0
                .iter()
                // FIXME: Code smell, we silently delete unknown references
                .filter_map(|(k, v)| Some((hir_map.get_hir_id(*k)?, hir_map.get_hir_id(*v)?)))
                .collect(),
        )
    }
}
