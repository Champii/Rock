use std::collections::BTreeMap;

use super::InferState;
use crate::ast::{PrimitiveType, Type};
use crate::hir::visit::*;
use crate::hir::*;
use crate::walk_list;

#[derive(Debug)]
pub struct AnnotateContext {
    state: InferState,
    body_arguments: BTreeMap<FnBodyId, Vec<ArgumentDecl>>,
}

impl AnnotateContext {
    pub fn new(state: InferState) -> Self {
        Self {
            state,
            body_arguments: BTreeMap::new(),
        }
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
        self.state
            .new_named_annotation(f.name.to_string(), f.hir_id.clone());

        self.body_arguments
            .insert(f.body_id.clone(), f.arguments.clone());
    }

    fn visit_fn_body(&mut self, fn_body: &FnBody) {
        let args = self.body_arguments.get(&fn_body.id).unwrap().clone();

        self.state.named_types.push();
        self.state.named_types_flat.push();

        walk_list!(self, visit_argument_decl, &args);

        self.visit_body(&fn_body.body);

        self.state.named_types.pop();
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

    fn visit_identifier_path(&mut self, id: &IdentifierPath) {
        self.visit_identifier(id.path.iter().last().unwrap());
    }

    fn visit_identifier(&mut self, id: &Identifier) {
        self.state
            .new_named_annotation(id.name.clone(), id.hir_id.clone());
    }

    fn visit_if(&mut self, r#if: &If) {
        self.state.new_type_id(r#if.hir_id.clone());

        self.visit_expression(&r#if.predicat);

        self.visit_body(&r#if.body);

        if let Some(e) = &r#if.else_ {
            self.visit_else(e);
        }
    }

    fn visit_function_call(&mut self, fc: &FunctionCall) {
        self.state.new_type_id(fc.hir_id.clone());

        self.visit_expression(&fc.op);

        walk_list!(self, visit_expression, &fc.args);
    }

    fn visit_native_operator(&mut self, op: &NativeOperator) {
        let t = match op.kind {
            NativeOperatorKind::Eq
            | NativeOperatorKind::GT
            | NativeOperatorKind::GE
            | NativeOperatorKind::LT
            | NativeOperatorKind::LE => PrimitiveType::Bool,
            _ => PrimitiveType::Int64,
        };

        self.state
            .new_type_solved(op.hir_id.clone(), Type::Primitive(t));
    }
}
