use std::collections::{HashMap, HashSet};

use crate::{ast::Trait, ty::Type};

#[derive(Debug, Default, Clone)]
pub struct TraitSolver {
    pub implemented_trait: HashMap<String, HashSet<String>>, // implementors -> trait
    pub trait_methods: HashMap<String, HashSet<String>>,     // trait/struct -> method_name
}

impl TraitSolver {
    pub fn new() -> TraitSolver {
        TraitSolver {
            implemented_trait: HashMap::new(),
            trait_methods: HashMap::new(),
        }
    }

    pub fn add_trait(&mut self, tr: &Trait) {
        self.trait_methods.insert(
            tr.name.get_name(),
            tr.defs
                .iter()
                .map(|fundecl| fundecl.name.to_string())
                .collect(),
        );
    }

    pub fn add_implementor(&mut self, implementor_type: Type, trait_type: Type) {
        self.implemented_trait
            .entry(implementor_type.get_name())
            .or_insert_with(HashSet::new)
            .insert(trait_type.get_name());
    }

    pub fn does_impl_fn(&self, implementor_type: &Type, fn_name: String) -> bool {
        let trait_associated = if let Some((trait_name, _)) = self
            .trait_methods
            .iter()
            .find(|(_trait_name, set)| set.contains(&fn_name))
        {
            trait_name.clone()
        } else {
            return false;
        };

        self.implemented_trait
            .get(&implementor_type.get_name())
            .unwrap()
            .contains(&trait_associated)
    }
}
