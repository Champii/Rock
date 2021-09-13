use std::collections::HashMap;

use crate::{ast::*, hir::HirId, NodeId};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HirMap {
    map: HashMap<HirId, NodeId>,
    rev_map: HashMap<NodeId, HirId>,
}

impl HirMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_hir_id(&mut self, identity: Identity) -> HirId {
        let hir_id = HirId::next();

        self.add_hir_mapping(hir_id.clone(), identity.node_id);

        hir_id
    }

    pub fn get_hir_id(&self, node_id: NodeId) -> Option<HirId> {
        self.rev_map.get(&node_id).cloned()
    }

    pub fn get_node_id(&self, hir_id: &HirId) -> Option<NodeId> {
        self.map.get(hir_id).cloned()
    }

    fn add_hir_mapping(&mut self, hir_id: HirId, node_id: NodeId) {
        self.map.insert(hir_id.clone(), node_id);

        self.rev_map.insert(node_id, hir_id);
    }

    pub fn duplicate_hir_mapping(&mut self, hir_id: HirId) -> Option<HirId> {
        let node_id = self.get_node_id(&hir_id)?;

        let mut fake_ident = Identity::new_placeholder();

        fake_ident.node_id = node_id;

        let new_id = self.next_hir_id(fake_ident);

        self.add_hir_mapping(new_id.clone(), node_id);

        Some(new_id)
    }
}
