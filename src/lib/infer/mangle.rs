use crate::{hir::visit_mut::*, hir::*, walk_list};
use std::collections::HashMap;

#[derive(Debug)]
pub struct MangleContext {
    pub trait_call_to_mangle: HashMap<HirId, Vec<String>>, // fc_call => prefixes
}

impl<'a> VisitorMut<'a> for MangleContext {
    fn visit_function_call(&mut self, fc: &'a mut FunctionCall) {
        if let Some(prefixes) = self.trait_call_to_mangle.get(&fc.hir_id) {
            fc.mangle(prefixes.clone());
        }

        self.visit_expression(&mut fc.op);

        walk_list!(self, visit_expression, &mut fc.args);
    }
}
