use std::collections::HashMap;

use crate::{
    ast::{tree::*, visit::*, NodeId},
    resolver::ResolutionMap,
};

#[derive(Debug, Default)]
pub struct UnusedCollector {
    resolutions: ResolutionMap<NodeId>,
    fn_list: HashMap<NodeId, bool>,
    method_list: HashMap<NodeId, bool>,
}

impl UnusedCollector {
    pub fn new(resolutions: ResolutionMap<NodeId>) -> Self {
        Self {
            resolutions,
            ..Default::default()
        }
    }

    // (fns, methods)
    pub fn take_unused(self) -> (Vec<NodeId>, Vec<NodeId>) {
        (
            self.fn_list
                .into_iter()
                .filter_map(|(id, used)| if !used { Some(id + 1) } else { None }) // +1 to get the
                // fn identifier's node_id, as it always follows the fn node_id
                .collect(),
            self.method_list
                .into_iter()
                .filter_map(|(id, used)| if !used { Some(id) } else { None })
                .collect(),
        )
    }
}

impl<'a> Visitor<'a> for UnusedCollector {
    fn visit_mod(&mut self, m: &'a Mod) {
        // We add every top level first

        for top in &m.top_levels {
            match &top {
                TopLevel::Prototype(_p) => {}
                TopLevel::Use(_u) => (),
                TopLevel::Trait(t) => {
                    for f in &t.defs {
                        self.method_list.insert(f.node_id, false);
                    }
                }
                TopLevel::Impl(_i) => {}
                TopLevel::Struct(_s) => {}
                TopLevel::Mod(_, _m) => (),
                TopLevel::Infix(_, _) => (),
                TopLevel::Function(f) => {
                    self.fn_list.insert(f.node_id, false);

                    if f.name.name == *"main" {
                        self.fn_list.insert(f.node_id, true);
                    }
                }
            }
        }

        walk_list!(self, visit_top_level, &m.top_levels);
    }

    fn visit_top_level(&mut self, top_level: &'a TopLevel) {
        match &top_level {
            TopLevel::Prototype(p) => self.visit_prototype(p),
            TopLevel::Use(_u) => (),
            TopLevel::Trait(t) => self.visit_trait(t),
            TopLevel::Impl(i) => self.visit_impl(i),
            TopLevel::Struct(i) => self.visit_struct_decl(i),
            TopLevel::Mod(name, m) => {
                self.visit_identifier(name);
                self.visit_mod(m);
            }
            TopLevel::Function(f) => self.visit_function_decl(f),
            TopLevel::Infix(_ident, _) => (),
        };
    }

    fn visit_prototype(&mut self, prototype: &'a Prototype) {
        self.visit_func_type(&prototype.signature);
    }

    fn visit_function_decl(&mut self, f: &'a FunctionDecl) {
        walk_list!(self, visit_identifier, &f.arguments);

        self.visit_body(&f.body);
    }

    fn visit_identifier(&mut self, id: &'a Identifier) {
        if let Some(reso) = self.resolutions.get_recur(&id.node_id) {
            if let Some(used) = self.fn_list.get_mut(&reso) {
                *used = true;
            } else if let Some(used) = self.method_list.get_mut(&reso) {
                *used = true;
            }
        }
    }
}

pub fn collect_unused(root: &Root) -> (Vec<NodeId>, Vec<NodeId>) {
    let mut unused_collector = UnusedCollector::new(root.resolutions.clone());

    unused_collector.visit_root(root);

    unused_collector.take_unused()
}
