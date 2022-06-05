use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ast::{Impl, NodeId},
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

    /* pub fn trait_of_fn(&self, implementor_type: &Type, fn_name: String) -> Option<String> {
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
            .map(|x| {
                println!("{:?}", x);
                x
            })
            .nth(0)
            .map(|(trait_name, _)| trait_name.clone())
    } */

    pub fn node_id_of_fn_implementor(
        &self,
        implementor_type: &Type,
        fn_name: String,
    ) -> Option<NodeId> {
        self.implemented_fns
            .get(&implementor_type.get_name())
            .and_then(|set| {
                println!("set ?? {:?}", set);
                set.iter()
                    .filter(|(_, name)| **name == fn_name)
                    .map(|x| {
                        println!("{:?}", x);
                        x
                    })
                    .nth(0)
                    .map(|(id, _)| id.clone())
            })
    }
}
