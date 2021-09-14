use std::collections::BTreeMap;

use crate::{ast::Type, hir::visit_mut::*, hir::*, walk_list};

#[derive(Debug)]
pub struct MangleContext {
    pub types: BTreeMap<HirId, Type>,
}

impl<'a> VisitorMut<'a> for MangleContext {
    fn visit_function_decl(&mut self, f: &'a mut FunctionDecl) {
        let t = self.types.get(&f.hir_id).unwrap();

        if let Type::FuncType(f_t) = t {
            f.mangle(f_t.to_prefixes());
        } else {
            panic!("Not a function {:?}", t);
        }
    }

    fn visit_function_call(&mut self, fc: &'a mut FunctionCall) {
        let t = self.types.get(&fc.op.get_hir_id()).unwrap();

        if let Type::FuncType(f_t) = t {
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
        types: root.node_types.clone(),
    }
    .visit_root(root);
}
