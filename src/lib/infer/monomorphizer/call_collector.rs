use std::collections::BTreeMap;

use crate::{hir::visit::*, hir::*};

pub type Calls = BTreeMap<(HirId, HirId), FunctionCall>; // (call_site_id, caller_id)

#[derive(Debug, Default)]
struct FnCallCollector {
    call_list: Calls,
    current_fn_hir_id: Option<HirId>,
    fns: BTreeMap<FnBodyId, HirId>,
}

impl FnCallCollector {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    // (fns, methods)
    pub fn take_call_list(self) -> Calls {
        self.call_list
    }

    pub fn run(mut self, root: &Root) -> Self {
        self.visit_root(root);

        self
    }
}

impl<'a> Visitor<'a> for FnCallCollector {
    fn visit_function_decl(&mut self, f: &'a FunctionDecl) {
        self.fns.insert(f.body_id.clone(), f.hir_id.clone());

        walk_function_decl(self, f);
    }

    fn visit_fn_body(&mut self, fn_body: &'a FnBody) {
        let f_hir_id = self.fns.get(&fn_body.id).unwrap();

        self.current_fn_hir_id = Some(f_hir_id.clone());

        walk_fn_body(self, fn_body);
    }

    fn visit_function_call(&mut self, fc: &'a FunctionCall) {
        self.call_list.insert(
            (self.current_fn_hir_id.clone().unwrap(), fc.op.get_hir_id()),
            fc.clone(),
        );

        walk_function_call(self, fc);
    }
}

pub fn collect_calls(root: &Root) -> Calls {
    FnCallCollector::new().run(root).take_call_list()
}
