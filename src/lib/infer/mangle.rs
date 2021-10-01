use std::collections::BTreeMap;

use crate::{hir::visit_mut::*, hir::*, ty::Type, walk_list};

#[derive(Debug)]
pub struct MangleContext {
    pub node_types: BTreeMap<HirId, Type>,
}

impl<'a> VisitorMut<'a> for MangleContext {
    fn visit_function_decl(&mut self, f: &'a mut FunctionDecl) {
        let t = self.node_types.get(&f.hir_id).unwrap();

        if let Type::Func(f_t) = t {
            f.mangle(f_t.to_prefixes());
        } else {
            panic!("Not a function {:?}", t);
        }
    }

    fn visit_function_call(&mut self, fc: &'a mut FunctionCall) {
        let t = self.node_types.get(&fc.op.get_hir_id()).unwrap();

        if let Type::Func(f_t) = t {
            fc.mangle(f_t.to_prefixes());

            self.visit_expression(&mut fc.op);

            walk_list!(self, visit_expression, &mut fc.args);
        } else {
            panic!("Not a function {:?}", t);
        }
    }
}

pub fn mangle(root: &mut Root) {
    MangleContext {
        node_types: root.node_types.clone(),
    }
    .visit_root(root);
}
