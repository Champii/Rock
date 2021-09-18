use std::collections::BTreeMap;

use crate::{
    ast::{resolve::ResolutionMap, FuncType, PrimitiveType, Type, TypeSignature},
    diagnostics::Diagnostics,
    hir::visit::*,
    hir::*,
    walk_list, Envs,
};

#[derive(Debug)]
struct ConstraintContext<'a> {
    hir: &'a Root,
    tmp_resolutions: BTreeMap<HirId, ResolutionMap<HirId>>,
    envs: Envs,
}

impl<'a> ConstraintContext<'a> {
    pub fn new(envs: Envs, hir: &'a Root) -> Self {
        Self {
            envs,
            hir,
            tmp_resolutions: BTreeMap::default(),
        }
    }

    pub fn constraint(&mut self, root: &'a Root) {
        let entry_point = root.get_function_by_name("main").unwrap();

        self.envs.set_current_fn((
            entry_point.hir_id.clone(),
            TypeSignature::default().with_ret(Type::int64()),
        ));

        self.tmp_resolutions
            .entry(entry_point.hir_id.clone())
            .or_insert_with(|| ResolutionMap::default());

        self.visit_function_decl(&entry_point);
    }

    pub fn get_envs(self) -> Envs {
        self.envs
    }

    pub fn resolve(&self, id: &HirId) -> Option<HirId> {
        match self
            .tmp_resolutions
            .get(&self.hir.type_envs.get_current_fn().0)
        {
            Some(env) => env.get(id).or_else(|| self.hir.resolutions.get(id)),
            None => self.hir.resolutions.get(id),
        }
    }

    pub fn resolve_rec(&self, id: &HirId) -> Option<HirId> {
        self.resolve(id)
            .and_then(|reso| self.resolve_rec(&reso).or(Some(reso)))
    }

    pub fn setup_call(&mut self, fc: &FunctionCall, call_hir_id: &HirId) {
        if let Some(top_id) = self.resolve(call_hir_id) {
            if let Some(reso) = self.hir.arena.get(&top_id) {
                match reso {
                    HirNode::Prototype(p) => {
                        self.setup_prototype_call(fc, p);
                    }
                    HirNode::FunctionDecl(f) => {
                        self.setup_function_call(fc, f);
                    }
                    HirNode::Identifier(id) => {
                        self.setup_identifier_call(fc, id);
                    }
                    _ => unimplemented!("Cannot call {:#?}", reso),
                }
            } else {
                panic!("NO ARENA ITEM FOR HIR={:?}", top_id);
            }
        } else {
            panic!("No reso hir_id: {:#?}", call_hir_id);
        }
    }

    pub fn setup_identifier_call(&mut self, fc: &FunctionCall, id: &Identifier) {
        self.setup_call(fc, &id.get_hir_id());
    }

    pub fn setup_prototype_call(&mut self, fc: &FunctionCall, p: &Prototype) {
        let old_f = self.envs.get_current_fn();

        self.envs
            .set_current_fn((p.hir_id.clone(), p.signature.clone()));
        self.envs.set_current_fn(old_f);

        self.visit_prototype(p);

        self.envs.set_type(&fc.get_hir_id(), &p.signature.ret);

        self.envs
            .set_type(&fc.op.get_hir_id(), &p.signature.to_func_type());
    }

    pub fn setup_function_call(&mut self, fc: &FunctionCall, f: &FunctionDecl) {
        let sig = f.signature.apply_partial_types(
            &f.arguments
                .iter()
                .enumerate()
                .into_iter()
                .map(|(i, arg)| {
                    let arg_id = &fc.args.get(i).unwrap().get_hir_id();
                    self.envs.get_type(arg_id).cloned().or_else(|| {
                        if let HirNode::FunctionDecl(f2) =
                            self.hir.arena.get(&self.resolve(&arg_id)?)?
                        {
                            self.tmp_resolutions
                                .entry(f2.hir_id.clone())
                                .or_insert_with(|| ResolutionMap::default())
                                .insert(arg.get_hir_id(), f2.hir_id.clone());

                            self.tmp_resolutions
                                .entry(f.hir_id.clone())
                                .or_insert_with(|| ResolutionMap::default())
                                .insert(arg.get_hir_id(), f2.hir_id.clone());

                            self.tmp_resolutions
                                .entry(self.envs.get_current_fn().0.clone())
                                .or_insert_with(|| ResolutionMap::default())
                                .insert(arg.get_hir_id(), f2.hir_id.clone());

                            self.envs.set_type(&arg_id, &f.signature.to_func_type());

                            Some(f.signature.to_func_type())
                        } else {
                            None
                        }
                    })
                })
                .collect(),
            None,
        );
        // println!("SETUP FN CALL {:#?} {:#?} {:#?}", fc, f, sig);

        if self.envs.get_current_fn().0 == f.hir_id {
            warn!("Recursion !");

            self.envs.set_type(&fc.get_hir_id(), &sig.ret);
            self.envs.set_type(&fc.op.get_hir_id(), &sig.to_func_type());

            return;
        }

        let old_f = self.envs.get_current_fn();

        // We change scope here
        self.envs.set_current_fn((f.hir_id.clone(), sig.clone()));

        self.tmp_resolutions
            .entry(f.hir_id.clone())
            .or_insert_with(|| ResolutionMap::default());

        //
        //
        self.visit_function_decl(f);
        //
        //

        let new_f_type = self.envs.get_type(&f.hir_id).unwrap().clone();

        let mut new_f_arg_types = vec![];
        let new_f_sig;

        let new_f_ret = if let Type::FuncType(new_f_type_inner) = &new_f_type.clone() {
            new_f_arg_types = new_f_type_inner
                .arguments
                .iter()
                .map(|arg| *arg.clone())
                .collect();

            new_f_sig = new_f_type_inner.to_type_signature();
            *new_f_type_inner.ret.clone()
        } else {
            new_f_sig = sig.clone();
            sig.ret.clone()
        };

        self.envs.amend_current_sig(&new_f_sig);

        // We restore the scope here
        self.envs.set_current_fn(old_f);

        self.envs.set_type(&fc.get_hir_id(), &new_f_ret);
        self.envs.set_type(&fc.op.get_hir_id(), &new_f_type);

        if let Some(reso) = self.resolve(&fc.op.get_hir_id()) {
            self.envs.set_type(&reso, &new_f_type);
        }

        // update args her
        fc.args.iter().enumerate().for_each(|(i, arg)| {
            if let Some(_reso_id) = self.resolve_rec(&arg.get_hir_id()) {
                self.envs
                    .set_type(&arg.get_hir_id().clone(), new_f_arg_types.get(i).unwrap());
            }
        });
    }
}

impl<'a, 'ar> Visitor<'a> for ConstraintContext<'ar> {
    fn visit_root(&mut self, _r: &'a Root) {}

    fn visit_top_level(&mut self, _t: &'a TopLevel) {}

    fn visit_trait(&mut self, _t: &'a Trait) {}

    fn visit_function_decl(&mut self, f: &'a FunctionDecl) {
        self.envs.apply_args_type(f);

        walk_function_decl(self, f);

        self.visit_fn_body(&self.hir.get_body(&f.body_id).unwrap());

        self.envs.set_type(
            &f.hir_id,
            &Type::FuncType(FuncType::new(
                f.name.name.clone(),
                f.arguments
                    .iter()
                    .map(|arg| self.envs.get_type(&arg.get_hir_id()).unwrap())
                    .cloned()
                    .collect(),
                self.envs
                    .get_type(&self.hir.get_body(&f.body_id).unwrap().get_hir_id())
                    .unwrap()
                    .clone(),
            )),
        );

        self.envs.set_type_eq(&f.name.hir_id, &f.hir_id);
        self.tmp_resolutions
            .get_mut(&self.envs.get_current_fn().0)
            .unwrap()
            .insert(f.name.hir_id.clone(), f.hir_id.clone());
    }

    fn visit_prototype(&mut self, p: &Prototype) {
        self.envs.set_type(&p.hir_id, &p.signature.to_func_type());

        self.tmp_resolutions
            .get_mut(&self.envs.get_current_fn().0)
            .unwrap()
            .insert(p.name.hir_id.clone(), p.hir_id.clone());

        walk_prototype(self, p);
    }

    fn visit_body(&mut self, body: &'a Body) {
        body.stmts
            .iter()
            .for_each(|stmt| self.visit_statement(&stmt));
    }

    fn visit_assign(&mut self, assign: &'a Assign) {
        self.visit_identifier(&assign.name);
        self.visit_expression(&assign.value);

        self.envs
            .set_type_eq(&assign.name.get_hir_id(), &assign.value.get_hir_id());
    }

    fn visit_if(&mut self, r#if: &'a If) {
        self.visit_expression(&r#if.predicat);

        self.visit_body(&r#if.body);

        self.envs.set_type_eq(&r#if.hir_id, &r#if.body.get_hir_id());

        if let Some(e) = &r#if.else_ {
            match &**e {
                Else::Body(b) => {
                    self.visit_body(b);
                    self.envs
                        .set_type_eq(&b.get_hir_id(), &r#if.body.get_hir_id());
                }
                Else::If(i) => {
                    self.visit_if(i);
                }
            }
        }
    }

    fn visit_expression(&mut self, expr: &'a Expression) {
        match &*expr.kind {
            ExpressionKind::Lit(lit) => self.visit_literal(&lit),
            ExpressionKind::Return(expr) => self.visit_expression(&expr),
            ExpressionKind::Identifier(id) => self.visit_identifier_path(&id),
            ExpressionKind::NativeOperation(op, left, right) => {
                self.visit_identifier(&left);
                self.visit_identifier(&right);

                //FIXME
                let arg_t = match op.kind {
                    NativeOperatorKind::IEq
                    | NativeOperatorKind::IGT
                    | NativeOperatorKind::IGE
                    | NativeOperatorKind::ILT
                    | NativeOperatorKind::ILE
                    | NativeOperatorKind::IAdd
                    | NativeOperatorKind::ISub
                    | NativeOperatorKind::IDiv
                    | NativeOperatorKind::IMul => PrimitiveType::Int64,
                    NativeOperatorKind::FEq
                    | NativeOperatorKind::FGT
                    | NativeOperatorKind::FGE
                    | NativeOperatorKind::FLT
                    | NativeOperatorKind::FLE
                    | NativeOperatorKind::FAdd
                    | NativeOperatorKind::FSub
                    | NativeOperatorKind::FDiv
                    | NativeOperatorKind::FMul => PrimitiveType::Float64,
                    NativeOperatorKind::BEq => PrimitiveType::Bool,
                };

                self.envs
                    .set_type(&left.hir_id.clone(), &Type::Primitive(arg_t.clone()));
                self.envs
                    .set_type(&right.hir_id.clone(), &Type::Primitive(arg_t));

                self.visit_native_operator(&op);
            }
            ExpressionKind::FunctionCall(fc) => {
                self.visit_expression(&fc.op);

                walk_list!(self, visit_expression, &fc.args);

                self.setup_call(&fc, &fc.op.get_hir_id());
            }
        }
    }

    fn visit_literal(&mut self, lit: &Literal) {
        let t = match &lit.kind {
            LiteralKind::Number(_n) => Type::Primitive(PrimitiveType::Int64),
            LiteralKind::Float(_f) => Type::Primitive(PrimitiveType::Float64),
            LiteralKind::String(_s) => Type::Primitive(PrimitiveType::String),
            LiteralKind::Bool(_b) => Type::Primitive(PrimitiveType::Bool),
        };

        self.envs.set_type(&lit.hir_id, &t);
    }

    fn visit_identifier_path(&mut self, id: &'a IdentifierPath) {
        self.visit_identifier(&id.path.iter().last().unwrap());
    }

    fn visit_identifier(&mut self, id: &Identifier) {
        if let Some(reso) = self.resolve_rec(&id.hir_id) {
            if self.envs.get_type(&reso).is_some() {
                self.envs.set_type_eq(&id.get_hir_id(), &reso);
            }
        } else {
            warn!("No identifier resolution {:?}", id);
        }
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

        self.envs.set_type(&op.hir_id, &Type::Primitive(t));
    }
}

pub fn solve<'a>(root: &mut Root) -> (BTreeMap<HirId, ResolutionMap<HirId>>, Diagnostics) {
    let diagnostics = Diagnostics::default();

    let infer_state = Envs::default();

    let mut constraint_ctx = ConstraintContext::new(infer_state, &root);

    constraint_ctx.constraint(&root);

    let tmp_resolutions = constraint_ctx.tmp_resolutions.clone();

    let envs = constraint_ctx.get_envs();

    root.type_envs = envs;

    // if let Err(diags) = infer_state.solve() {
    //     for diag in diags {
    //         diagnostics.push_error(diag.clone());
    //     }
    // }

    (tmp_resolutions, diagnostics)
}
