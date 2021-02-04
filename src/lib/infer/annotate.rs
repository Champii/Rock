use super::InferState;
use crate::ast::{PrimitiveType, Type};
use crate::hir::visit::*;
use crate::hir::*;

#[derive(Debug)]
pub struct AnnotateContext {
    state: InferState,
}

impl AnnotateContext {
    pub fn new(state: InferState) -> Self {
        Self { state }
    }

    pub fn annotate(&mut self, root: &Root) {
        self.visit_root(root);
    }

    pub fn get_state(self) -> InferState {
        self.state
    }
}

impl<'a> Visitor<'a> for AnnotateContext {
    fn visit_function_decl(&mut self, f: &FunctionDecl) {
        // self.visit_identifier(&f.name);
        self.state
            .new_named_annotation(f.name.to_string(), f.hir_id.clone());

        walk_list!(self, visit_argument_decl, &f.arguments);
    }

    fn visit_literal(&mut self, lit: &Literal) {
        match &lit.kind {
            LiteralKind::Number(_n) => self
                .state
                .new_type_solved(lit.hir_id.clone(), Type::Primitive(PrimitiveType::Int64)),
            LiteralKind::String(s) => self.state.new_type_solved(
                lit.hir_id.clone(),
                Type::Primitive(PrimitiveType::String(s.len())),
            ),
            LiteralKind::Bool(_b) => self
                .state
                .new_type_solved(lit.hir_id.clone(), Type::Primitive(PrimitiveType::Bool)),
        };
    }

    fn visit_identifier(&mut self, id: &Identifier) {
        self.state
            .new_named_annotation(id.name.clone(), id.hir_id.clone());
    }
}
