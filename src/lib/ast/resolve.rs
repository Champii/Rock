use std::collections::HashMap;

use crate::{
    ast::visit::*, ast::*, ast_lowering::HirMap, diagnostics::Diagnostic, hir::HirId,
    parser::ParsingCtx, scopes::*, NodeId,
};

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

#[derive(Debug)]
pub struct ResolveCtx<'a> {
    pub parsing_ctx: &'a mut ParsingCtx,
    scopes: Scopes<String, Identity>,
    resolutions: ResolutionMap<NodeId>,
}

impl<'a> Visitor<'a> for ResolveCtx<'a> {
    fn visit_mod(&mut self, m: &'a Mod) {
        // We add every top level first
        for top in &m.top_levels {
            match &top.kind {
                TopLevelKind::Mod(_, m) => (),
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
            TopLevelKind::Mod(_, m) => walk_mod(self, m),
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
            None => self
                .parsing_ctx
                .diagnostics
                .push(Diagnostic::new_unknown_identifier(id.identity.span.clone())),
        };
    }
}

pub fn resolve(root: &mut Root, parsing_ctx: &mut ParsingCtx) {
    let mut ctx = ResolveCtx {
        parsing_ctx,
        scopes: Scopes::new(),
        resolutions: ResolutionMap::default(),
    };

    ctx.visit_root(root);

    root.resolutions = ctx.resolutions;
}
