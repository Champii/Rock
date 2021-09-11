use std::collections::{BTreeMap, HashMap};

use super::InferState;
use crate::hir::visit::*;
use crate::hir::*;
use crate::walk_list;
use crate::{
    ast::{PrimitiveType, Type, TypeSignature},
    walk_map,
};

#[derive(Debug)]
pub struct AnnotateContext<'a> {
    state: InferState<'a>,
    trait_methods: HashMap<String, HashMap<TypeSignature, FunctionDecl>>,
    body_arguments: BTreeMap<FnBodyId, Vec<ArgumentDecl>>,
}

impl<'a> AnnotateContext<'a> {
    pub fn new(
        state: InferState<'a>,
        trait_methods: HashMap<String, HashMap<TypeSignature, FunctionDecl>>,
    ) -> Self {
        Self {
            state,
            trait_methods,
            body_arguments: BTreeMap::new(),
        }
    }

    pub fn annotate(&mut self, root: &'a Root) {
        self.visit_root(root);
    }

    pub fn get_state(self) -> InferState<'a> {
        self.state
    }
}

impl<'a, 'ar> Visitor<'a> for AnnotateContext<'ar> {
    fn visit_root(&mut self, root: &'a Root) {
        walk_map!(self, visit_top_level, &root.top_levels);

        for (_, r#trait) in &root.traits {
            self.visit_trait(r#trait);
        }

        for (_, impls) in &root.trait_methods {
            walk_map!(self, visit_function_decl, impls);
        }

        walk_map!(self, visit_fn_body, &root.bodies);
    }

    fn visit_top_level(&mut self, top_level: &'a TopLevel) {
        match &top_level.kind {
            TopLevelKind::Prototype(p) => {
                self.state.new_type_id(p.name.get_hir_id());
                self.visit_prototype(&p);
            }
            TopLevelKind::Function(f) => {
                self.state.new_type_id(f.name.get_hir_id());
                self.visit_function_decl(&f);
            }
        };
    }

    // Ignoring traits
    fn visit_trait(&mut self, _t: &Trait) {}

    fn visit_impl(&mut self, i: &Impl) {
        self.visit_type(&i.name);

        walk_list!(self, visit_type, &i.types);

        walk_list!(self, visit_function_decl, &i.defs);

        for (_method_name, list) in &self.trait_methods {
            for (sig, f_decl) in list {
                for (i, arg) in f_decl.arguments.iter().enumerate() {
                    self.state
                        .new_type_solved(arg.name.hir_id.clone(), sig.args[i].clone());
                }

                self.state
                    .new_type_solved(f_decl.hir_id.clone(), sig.ret.clone());
            }
        }
    }

    fn visit_function_decl(&mut self, f: &FunctionDecl) {
        self.state.new_type_id(f.hir_id.clone());
        self.state.new_type_id(f.name.hir_id.clone());

        self.body_arguments
            .insert(f.body_id.clone(), f.arguments.clone());
    }

    fn visit_prototype(&mut self, p: &Prototype) {
        self.state.new_type_id(p.hir_id.clone());
        self.state.new_type_id(p.name.hir_id.clone());
    }

    fn visit_fn_body(&mut self, fn_body: &FnBody) {
        let args = self.body_arguments.get(&fn_body.id).unwrap().clone();

        walk_list!(self, visit_argument_decl, &args);

        self.visit_body(&fn_body.body);
    }

    fn visit_literal(&mut self, lit: &Literal) {
        let t = match &lit.kind {
            LiteralKind::Number(_n) => Type::Primitive(PrimitiveType::Int64),
            LiteralKind::Float(_f) => Type::Primitive(PrimitiveType::Float64),
            LiteralKind::String(s) => Type::Primitive(PrimitiveType::String(s.len())),
            LiteralKind::Bool(_b) => Type::Primitive(PrimitiveType::Bool),
        };

        self.state.new_type_solved(lit.hir_id.clone(), t);
    }

    fn visit_identifier_path(&mut self, id: &IdentifierPath) {
        self.visit_identifier(id.path.iter().last().unwrap());
    }

    fn visit_identifier(&mut self, id: &Identifier) {
        self.state.new_type_id(id.hir_id.clone());
    }

    fn visit_if(&mut self, r#if: &If) {
        self.state.new_type_id(r#if.hir_id.clone());

        self.visit_expression(&r#if.predicat);

        self.state.solve_type(
            r#if.predicat.get_terminal_hir_id(),
            Type::Primitive(PrimitiveType::Bool),
        );

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
            NativeOperatorKind::IEq
            | NativeOperatorKind::IGT
            | NativeOperatorKind::IGE
            | NativeOperatorKind::ILT
            | NativeOperatorKind::ILE
            | NativeOperatorKind::FEq
            | NativeOperatorKind::FGT
            | NativeOperatorKind::FGE
            | NativeOperatorKind::FLT
            | NativeOperatorKind::FLE
            | NativeOperatorKind::BEq => PrimitiveType::Bool,
            NativeOperatorKind::IAdd
            | NativeOperatorKind::ISub
            | NativeOperatorKind::IDiv
            | NativeOperatorKind::IMul => PrimitiveType::Int64,
            NativeOperatorKind::FAdd
            | NativeOperatorKind::FSub
            | NativeOperatorKind::FDiv
            | NativeOperatorKind::FMul => PrimitiveType::Float64,
        };

        self.state
            .new_type_solved(op.hir_id.clone(), Type::Primitive(t));
    }
}
