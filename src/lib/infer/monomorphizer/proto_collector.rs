use std::collections::HashMap;

use crate::{
    ast::{Type, TypeSignature},
    hir::HirId,
    InferState,
};
use crate::{hir::visit::*, hir::*};

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

    // (fns, methods)
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
    let mut protos = ProtoCollector::new().run(root).take_proto_list();

    // solve_existing_types(&mut protos, infer_state);

    protos
}

// fn solve_existing_types(protos: &mut Protos, infer_state: &InferState) {
//     for (hir_id, t_sig) in protos {
//         let t = infer_state.get_type(infer_state.get_type_id(hir_id.clone()).unwrap());

//         if let Some(Type::FuncType(f)) = t {
//             t_sig.apply_partial_types_mut(
//                 &f.arguments
//                     .iter()
//                     .map(|arg| infer_state.get_type(*arg))
//                     .collect(),
//                 infer_state.get_type(f.ret),
//             );
//         }
//     }
// }
