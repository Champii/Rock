use std::collections::HashMap;

use crate::{ast::TypeSignature, hir::visit::*, hir::*};

pub type Protos = HashMap<HirId, TypeSignature>;

#[derive(Debug, Default)]
struct ProtoCollector {
    proto_list: Protos,
}

impl ProtoCollector {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn take_proto_list(self) -> Protos {
        self.proto_list
    }

    pub fn run(mut self, root: &Root) -> Self {
        self.visit_root(root);

        self
    }
}

impl<'a> Visitor<'a> for ProtoCollector {
    fn visit_prototype(&mut self, p: &'a Prototype) {
        if p.signature.is_solved() {
            self.proto_list.insert(p.get_hir_id(), p.signature.clone());
        }

        walk_prototype(self, p);
    }

    fn visit_function_decl(&mut self, f: &'a FunctionDecl) {
        self.proto_list.insert(f.get_hir_id(), f.signature.clone());

        walk_function_decl(self, f);
    }
}

pub fn collect_prototypes(root: &Root) -> Protos {
    ProtoCollector::new().run(root).take_proto_list()
}
