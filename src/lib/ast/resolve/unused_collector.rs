use std::collections::HashMap;

use crate::NodeId;
use crate::{ast::resolve::ResolutionMap, ast::visit::*};
use crate::{ast::*, walk_list};

#[derive(Debug, Default)]
pub struct UnusedCollector {
    // hir: &'a Root,
    // state: InferState,
    // current_body: Option<FnBodyId>,
    // new_resolutions: HashMap<HirId, HirId>,
    resolutions: ResolutionMap<NodeId>,
    list: HashMap<NodeId, bool>,
}

impl UnusedCollector {
    pub fn new(resolutions: ResolutionMap<NodeId>) -> Self {
        Self {
            resolutions,
            ..Default::default()
        }
    }

    pub fn take_unused(self) -> Vec<NodeId> {
        self.list
            .into_iter()
            .filter_map(|(id, used)| if !used { Some(id) } else { None })
            .collect()
    }
}

impl<'a> Visitor<'a> for UnusedCollector {
    fn visit_mod(&mut self, m: &'a Mod) {
        // We add every top level first

        for top in &m.top_levels {
            match &top.kind {
                TopLevelKind::Prototype(p) => {
                    // self.add_to_current_scope((*p.name).clone(), p.identity.clone());
                }
                TopLevelKind::Use(_u) => (),
                TopLevelKind::Trait(t) => {
                    for f in &t.defs {
                        println!("Trait method {:?}", f.identity);

                        self.list.insert(f.identity.node_id.clone(), false);
                    }
                    // for proto in &t.defs {
                    //     self.add_to_current_scope((*proto.name).clone(), proto.identity.clone());
                    // }
                }
                TopLevelKind::Impl(i) => {
                    // for f in &i.defs {
                    //     println!("Impl method {:?}", f.identity);
                    //     self.list.insert(f.identity.node_id.clone(), false);
                    // }
                }
                TopLevelKind::Mod(_, _m) => (),
                TopLevelKind::Infix(_, _) => (),
                TopLevelKind::Function(f) => {
                    // self.add_to_current_scope((*f.name).clone(), f.identity.clone());
                    self.list.insert(f.identity.node_id.clone(), false);

                    if f.name.name == "main".to_string() {
                        self.list.insert(f.identity.node_id.clone(), true);
                    }
                    // self.list.insert(f.name.identity.node_id.clone(), false);
                    // println!("FN: {:#?}", f);
                }
            }
        }

        walk_list!(self, visit_top_level, &m.top_levels);
    }

    fn visit_top_level(&mut self, top_level: &'a TopLevel) {
        match &top_level.kind {
            TopLevelKind::Prototype(p) => self.visit_prototype(&p),
            TopLevelKind::Use(u) => (),
            TopLevelKind::Trait(t) => self.visit_trait(&t),
            TopLevelKind::Impl(i) => self.visit_impl(&i),
            TopLevelKind::Mod(name, m) => {
                self.visit_identifier(&name);
                self.visit_mod(&m);
            }
            TopLevelKind::Function(f) => self.visit_function_decl(&f),
            TopLevelKind::Infix(ident, _) => (),
        };
    }

    fn visit_prototype(&mut self, prototype: &'a Prototype) {
        self.visit_type_signature(&prototype.signature);
    }

    fn visit_function_decl(&mut self, f: &'a FunctionDecl) {
        // self.list.insert(f.identity.node_id.clone(), false);
        // // self.list.insert(f.name.identity.node_id.clone(), false);
        // println!("FN: {:#?}", f);

        // walk_function_decl(self, f);

        walk_list!(self, visit_argument_decl, &f.arguments);

        self.visit_body(&f.body);
    }

    fn visit_identifier(&mut self, id: &'a Identifier) {
        // println!("ID {:?}", id);
        if let Some(reso) = self.resolutions.get_recur(&id.identity.node_id) {
            // println!("RESO {:?}", reso);
            if let Some(used) = self.list.get_mut(&reso) {
                println!("WESH {:?} {:?} {:?}", id, reso, used);
                *used = true;
            }
        }
    }
}
