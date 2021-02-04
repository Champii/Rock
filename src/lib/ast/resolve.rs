use std::collections::HashMap;

use crate::{ast::visit::*, ast::*, ast_lowering::HirMap, hir::HirId, scopes::*, NodeId};

#[derive(Clone, Default, Debug)]
pub struct ResolutionMap<T>(HashMap<T, T>)
where
    T: Eq + Clone + std::hash::Hash;

impl<T: Eq + Clone + std::hash::Hash> ResolutionMap<T> {
    pub fn insert(&mut self, pointer_id: T, pointee_id: T) {
        self.0.insert(pointer_id, pointee_id);
    }

    pub fn get(&self, pointer_id: T) -> Option<T> {
        self.0.get(&pointer_id).cloned()
    }
}

impl ResolutionMap<NodeId> {
    pub fn lower_resolution_map(&self, hir_map: &HirMap) -> ResolutionMap<HirId> {
        ResolutionMap(
            self.0
                .iter()
                .map(|(k, v)| {
                    (
                        hir_map.get_hir_id(*k).unwrap(),
                        hir_map.get_hir_id(*v).unwrap(),
                    )
                })
                .collect(),
        )
    }
}

#[derive(Debug, Default)]
pub struct ResolveCtx {
    scopes: Scopes<String, Identity>,
    resolutions: ResolutionMap<NodeId>,
}

impl<'a> Visitor<'a> for ResolveCtx {
    fn visit_mod(&mut self, m: &'a Mod) {
        // We add every top level first
        for top in &m.top_levels {
            match &top.kind {
                TopLevelKind::Function(f) => {
                    self.scopes.add((*f.name).clone(), f.identity.clone());
                }
            }
        }

        walk_mod(self, m);
    }

    fn visit_top_level(&mut self, top: &'a TopLevel) {
        match &top.kind {
            TopLevelKind::Function(f) => {
                self.scopes.push();

                walk_function_decl(self, f);

                self.scopes.pop();
            }
        };
    }

    fn visit_argument_decl(&mut self, arg: &'a ArgumentDecl) {
        self.scopes.add(arg.name.clone(), arg.identity.clone());
    }

    fn visit_identifier(&mut self, id: &'a Identifier) {
        match self.scopes.get((*id).to_string()) {
            Some(pointed) => self
                .resolutions
                .insert(id.identity.node_id, pointed.node_id),
            None => panic!("Error undefined name {:?}", id),
        };
    }
}

pub fn resolve(root: &mut Root) {
    let mut ctx = ResolveCtx::default();

    ctx.visit_root(root);

    println!("RESOLUTION {:#?}", ctx);

    root.resolutions = ctx.resolutions;
}
