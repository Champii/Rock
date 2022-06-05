use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::{
    ast::{Impl, NodeId, Trait},
    ty::Type,
};

#[derive(Debug, Default, Clone)]
pub struct TraitSolver {
    pub implemented_trait: BTreeMap<String, BTreeSet<String>>, // implementors -> trait
    pub implemented_fns: BTreeMap<String, BTreeMap<NodeId, String>>, // implementor -> (fn_hir_id, fn_name)
    pub trait_methods: BTreeMap<String, BTreeMap<NodeId, String>>, // trait/struct -> (method_hir_id, method_name)
}

impl TraitSolver {
    pub fn new() -> TraitSolver {
        TraitSolver {
            implemented_trait: BTreeMap::new(),
            trait_methods: BTreeMap::new(),
            implemented_fns: BTreeMap::new(),
        }
    }

    pub fn add_trait(&mut self, tr: &Trait) {
        /* self.trait_methods.entry(tr.name.get_name()).or_insert(
            tr.defs
                .iter()
                .map(|fundecl| (fundecl.name.to_string(), fundecl.node_id))
                .collect(),
        ); */
    }

    pub fn add_impl(&mut self, tr: &Impl) {
        let effective_type = if tr.types.is_empty() {
            tr.name.get_name()
        } else {
            tr.types[0].get_name()
        };

        self.implemented_fns
            .entry(effective_type)
            .or_insert(BTreeMap::new())
            .extend(
                tr.defs
                    .iter()
                    .map(|fundecl| (fundecl.node_id, fundecl.name.name.clone()))
                    .collect::<Vec<_>>(),
            );

        self.trait_methods
            .entry(tr.name.get_name())
            .or_insert(BTreeMap::new())
            .extend(
                tr.defs
                    .iter()
                    .map(|fundecl| (fundecl.node_id, fundecl.name.to_string()))
                    .collect::<Vec<_>>(),
            );
    }

    pub fn add_implementor(&mut self, implementor_type: Type, trait_type: Type) {
        self.implemented_trait
            .entry(implementor_type.get_name())
            .or_insert_with(BTreeSet::new)
            .insert(trait_type.get_name());
    }

    /* pub fn does_impl_fn(&self, implementor_type: &Type, fn_name: String) -> bool {
           let trait_associated = if let Some((trait_name, _)) = self
               .trait_methods
               .iter()
               .find(|(_trait_name, set)| set.values().find(|k| **k == fn_name).is_some())
           {
               trait_name.clone()
           } else {
               return false;
           };

           self.implemented_trait
               .get(&implementor_type.get_name())
               .map(|set| set.contains(&trait_associated))
               .unwrap_or(false)
       }
    */
    pub fn trait_of_fn(&self, implementor_type: &Type, fn_name: String) -> Option<String> {
        self.trait_methods
            .iter()
            .filter(|(_trait_name, set)| set.values().find(|k| **k == fn_name).is_some())
            .filter(|(trait_name, _set)| {
                self.implemented_trait
                    .get(&implementor_type.get_name())
                    .map(|set| set.contains(trait_name.clone()))
                    .unwrap_or(false)
            })
            // TODO: Assuming we failed earlier if there were multiple traits implementing the same fn
            .nth(0)
            .map(|(trait_name, _)| trait_name.clone())
    }

    pub fn node_id_of_fn_implementor(
        &self,
        implementor_type: &Type,
        fn_name: String,
    ) -> Option<NodeId> {
        self.implemented_fns
            .get(&implementor_type.get_name())
            .and_then(|set| {
                set.iter()
                    .find(|(_, name)| **name == fn_name)
                    .map(|(id, _)| id.clone())
            })
    }
}
