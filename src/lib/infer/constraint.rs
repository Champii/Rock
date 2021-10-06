use std::collections::BTreeMap;

use crate::{
    diagnostics::{Diagnostic, Diagnostics},
    hir::visit::*,
    hir::*,
    infer::Envs,
    resolver::ResolutionMap,
    ty::{FuncType, PrimitiveType, Type},
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

    pub fn add_tmp_resolution_to_current_fn(&mut self, source: &HirId, dest: &HirId) {
        self.tmp_resolutions
            .entry(self.envs.get_current_fn().0)
            .or_insert_with(ResolutionMap::default)
            .insert(source.clone(), dest.clone());
    }

    pub fn constraint(&mut self, root: &'a Root) {
        let entry_point = root.get_function_by_name("main").unwrap();

        if !self.envs.set_current_fn((
            entry_point.hir_id.clone(),
            FuncType::default().with_ret(Type::int64()),
        )) {
            return;
        }

        self.visit_function_decl(&entry_point);
    }

    pub fn get_envs(self) -> Envs {
        self.envs
    }

    pub fn resolve(&self, id: &HirId) -> Option<HirId> {
        self.hir.resolutions.get(id).or_else(|| {
            self.tmp_resolutions
                .get(&self.envs.get_current_fn().0)
                .and_then(|env| env.get(id))
        })
    }

    pub fn resolve_rec(&self, id: &HirId) -> Option<HirId> {
        self.resolve(id)
            .and_then(|reso| self.resolve_rec(&reso).or(Some(reso)))
    }

    pub fn resolve_and_get(&self, hir: &HirId) -> Option<&HirNode> {
        self.hir
            .arena
            .get(
                &self
                    .resolve(hir)
                    .or_else(|| panic!("NO RESO FOR {:?}", hir))?,
            )
            .or_else(|| panic!("NO ARENA ITEM FOR {:?}", hir))
    }

    // FIXME: This is ugly
    pub fn setup_call(&mut self, fc: &FunctionCall, call_hir_id: &HirId) {
        self.resolve_and_get(call_hir_id)
            .cloned()
            .and_then(|reso| match reso {
                HirNode::Prototype(p) => self
                    .hir
                    .trait_methods
                    .get(&p.name.name)
                    .or_else(|| {
                        self.setup_prototype_call(fc, &p);

                        None
                    })
                    .and_then(|existing_impls| {
                        let new_sig = fc
                            .to_func_type(self.envs.get_current_env().unwrap())
                            .merge_with(&p.signature);

                        self.hir
                            .get_trait_method(p.name.name.clone(), &new_sig)
                            .or_else(|| {
                                self.envs.diagnostics.push_error(
                                    Diagnostic::new_unresolved_trait_call(
                                        self.envs.spans.get(&call_hir_id.clone()).unwrap().clone(),
                                        call_hir_id.clone(),
                                        new_sig,
                                        existing_impls.keys().cloned().collect(),
                                    ),
                                );

                                None
                            })
                    })
                    .map(|f| {
                        self.setup_trait_call(fc, &f);
                    }),
                HirNode::FunctionDecl(f) => {
                    self.setup_function_call(fc, &f);

                    Some(())
                }
                HirNode::Identifier(id) => {
                    self.setup_identifier_call(fc, &id);

                    Some(())
                }
                _ => unimplemented!("Cannot call {:#?}", reso),
            });
    }

    pub fn setup_trait_call(&mut self, fc: &FunctionCall, f: &FunctionDecl) {
        self.add_tmp_resolution_to_current_fn(&fc.op.get_hir_id(), &f.hir_id);

        self.setup_function_call(fc, f);
    }

    pub fn setup_identifier_call(&mut self, fc: &FunctionCall, id: &Identifier) {
        self.setup_call(fc, &id.get_hir_id());
    }

    pub fn setup_prototype_call(&mut self, fc: &FunctionCall, p: &Prototype) {
        let old_f = self.envs.get_current_fn();

        if !self
            .envs
            .set_current_fn((p.hir_id.clone(), p.signature.clone()))
        {
            return;
        }

        if !self.envs.set_current_fn(old_f) {
            return;
        }

        self.visit_prototype(p);

        self.envs.set_type(&fc.get_hir_id(), &p.signature.ret);

        self.envs
            .set_type(&fc.op.get_hir_id(), &Type::Func(p.signature.clone()));
    }

    // FIXME: This is ugly as well
    pub fn setup_function_call(&mut self, fc: &FunctionCall, f: &FunctionDecl) {
        if f.signature.arguments.len() != fc.args.len() {
            self.envs
                .diagnostics
                .push_error(Diagnostic::new_type_conflict(
                    self.envs.spans.get(&fc.op.get_hir_id()).unwrap().clone(),
                    fc.to_func_type(self.envs.get_current_env().unwrap()).into(),
                    f.signature.clone().into(),
                    fc.to_func_type(self.envs.get_current_env().unwrap()).into(),
                    f.signature.clone().into(),
                ));

            return;
        }

        // Creating a fresh signature by merging arguments types with function signature
        let sig = f.signature.apply_partial_types(
            &f.arguments
                .iter()
                .enumerate()
                .into_iter()
                .map(|(i, arg)| {
                    // Here we check if the argument is a function
                    // in order to set the proper resolution
                    let arg_id = &fc.args.get(i).unwrap().get_hir_id();

                    self.envs.get_type(arg_id).cloned().or_else(|| {
                        if let HirNode::FunctionDecl(f2) =
                            self.hir.arena.get(&self.resolve(arg_id)?)?
                        {
                            // Solving the func arg in the scope of the arg
                            // Adds a link like `arg` => `out fn` where the arg is defined
                            self.tmp_resolutions
                                .entry(f.hir_id.clone())
                                .or_insert_with(ResolutionMap::default)
                                .insert(arg.get_hir_id(), f2.hir_id.clone());

                            self.envs.set_type(arg_id, &f.signature.clone().into());

                            Some(f.signature.clone().into())
                        } else {
                            None
                        }
                    })
                })
                .collect::<Vec<_>>(),
            None,
        );

        if sig.arguments
            != fc
                .to_func_type(self.envs.get_current_env().unwrap())
                .arguments
        {
            self.envs
                .diagnostics
                .push_error(Diagnostic::new_type_conflict(
                    self.envs.spans.get(&fc.op.get_hir_id()).unwrap().clone(),
                    fc.to_func_type(self.envs.get_current_env().unwrap()).into(),
                    sig.clone().into(),
                    fc.to_func_type(self.envs.get_current_env().unwrap()).into(),
                    sig.clone().into(),
                ));

            return;
        }

        // Carring about recursion
        if self.envs.get_current_fn().0 == f.hir_id {
            warn!("Recursion ! {:#?}", sig);

            // Setting the proper call's types
            self.envs.set_type_eq(
                &fc.get_hir_id(),
                &self.hir.bodies.get(&f.body_id).unwrap().get_hir_id(),
            );

            self.envs.set_type(&fc.op.get_hir_id(), &sig.into());

            return;
        }

        // Saving the current function (id,sig)
        let old_f = self.envs.get_current_fn();

        // We change scope here
        if !self.envs.set_current_fn((f.hir_id.clone(), sig.clone())) {
            return;
        }

        // Create empty scope
        // TODO: might be unnecessary
        self.tmp_resolutions
            .entry(f.hir_id.clone())
            .or_insert_with(ResolutionMap::default);

        // We go down the rabbit hole
        //
        self.visit_function_decl(f);
        //
        // Annnd out we go !

        // Retrieve the newly defined function type
        let new_f_type = self.envs.get_type(&f.hir_id).unwrap().clone();

        let mut new_f_arg_types = vec![];
        let new_f_sig;

        // Get the func return type either
        // if it has been defined by the callee
        // or we take the sig's one
        let new_f_ret = if let Type::Func(new_f_type_inner) = &new_f_type.clone() {
            new_f_arg_types = new_f_type_inner.arguments.to_vec();

            new_f_sig = new_f_type_inner.clone();
            *new_f_type_inner.ret.clone()
        } else {
            new_f_sig = sig.clone();
            *sig.ret
        };

        // Fix the current sig if some types were still unknown
        self.envs.amend_current_sig(&new_f_sig);

        // We restore the scope here
        if !self.envs.set_current_fn(old_f) {
            return;
        }

        // Setting the proper call's types
        self.envs.set_type(&fc.get_hir_id(), &new_f_ret);
        self.envs.set_type(&fc.op.get_hir_id(), &new_f_type);

        // Setting up the calling identifier's type if one
        if let Some(reso) = self.resolve(&fc.op.get_hir_id()) {
            if let HirNode::Identifier(_) = self.hir.arena.get(&reso).unwrap() {
                self.envs.set_type(&reso, &new_f_type);
            }
        }

        // Set the call's arguments based on fn type.
        // This is only for type-checking purpose
        fc.args.iter().enumerate().for_each(|(i, arg)| {
            if let Some(_reso_id) = self.resolve_rec(&arg.get_hir_id()) {
                self.envs
                    .set_type(&arg.get_hir_id(), new_f_arg_types.get(i).unwrap());
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

        walk_list!(self, visit_argument_decl, &f.arguments);

        self.visit_fn_body(self.hir.get_body(&f.body_id).unwrap());

        self.envs.set_type(
            &f.hir_id,
            &Type::Func(FuncType::new(
                f.arguments
                    .iter()
                    .map(|arg| self.envs.get_type(&arg.get_hir_id()).unwrap())
                    .cloned()
                    .collect(),
                self.envs
                    .get_type(&self.hir.get_body(&f.body_id).unwrap().get_hir_id())
                    .cloned()
                    .or_else(|| Some(Type::forall("z")))
                    .unwrap(),
            )),
        );

        self.envs.set_type_eq(&f.name.hir_id, &f.hir_id);

        self.add_tmp_resolution_to_current_fn(&f.name.hir_id, &f.hir_id);
    }

    fn visit_prototype(&mut self, p: &Prototype) {
        if p.signature.is_solved() {
            self.envs.set_type(&p.hir_id, &p.signature.clone().into());
        }

        self.add_tmp_resolution_to_current_fn(&p.name.hir_id, &p.hir_id);

        walk_prototype(self, p);
    }

    fn visit_struct_decl(&mut self, s: &StructDecl) {
        let t = s.into();

        self.envs.set_type(&s.hir_id, &t);

        let struct_t = t.as_struct_type();

        s.defs.iter().for_each(|p| {
            self.envs
                .set_type(&p.hir_id, struct_t.defs.get(&p.name.name).unwrap());
        });
    }

    fn visit_struct_ctor(&mut self, s: &StructCtor) {
        let s_decl = self.hir.structs.get(&s.name.get_name()).unwrap();

        self.visit_struct_decl(s_decl);

        let t = s_decl.into();

        self.envs.set_type(&s.hir_id, &t);

        let struct_t = t.as_struct_type();

        walk_map!(self, visit_expression, &s.defs);

        s.defs.iter().for_each(|(k, expr)| {
            let declared_type = struct_t.defs.get(&k.name).unwrap();

            declared_type.is_func().then(|| {
                self.envs.get_type(&expr.get_hir_id()).cloned().or_else(|| {
                    if let HirNode::FunctionDecl(f2) =
                        self.hir.arena.get(&self.resolve(&expr.get_hir_id())?)?
                    {
                        self.add_tmp_resolution_to_current_fn(&k.get_hir_id(), &f2.hir_id);
                    }

                    None
                });
            });

            self.envs.set_type(&expr.get_hir_id(), declared_type);
        });
    }

    fn visit_body(&mut self, body: &'a Body) {
        body.stmts
            .iter()
            .for_each(|stmt| self.visit_statement(stmt));
    }

    fn visit_assign(&mut self, assign: &'a Assign) {
        self.visit_expression(&assign.value);
        self.visit_assign_left_side(&assign.name);

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
            ExpressionKind::Lit(lit) => self.visit_literal(lit),
            ExpressionKind::Return(expr) => self.visit_expression(expr),
            ExpressionKind::Identifier(id) => self.visit_identifier_path(id),
            ExpressionKind::StructCtor(s) => self.visit_struct_ctor(s),
            ExpressionKind::NativeOperation(op, left, right) => {
                self.visit_identifier(left);
                self.visit_identifier(right);

                //FIXME: Put this in another func
                let arg_t = match op.kind {
                    NativeOperatorKind::IEq
                    | NativeOperatorKind::Igt
                    | NativeOperatorKind::Ige
                    | NativeOperatorKind::Ilt
                    | NativeOperatorKind::Ile
                    | NativeOperatorKind::IAdd
                    | NativeOperatorKind::ISub
                    | NativeOperatorKind::IDiv
                    | NativeOperatorKind::IMul => PrimitiveType::Int64,
                    NativeOperatorKind::FEq
                    | NativeOperatorKind::Fgt
                    | NativeOperatorKind::Fge
                    | NativeOperatorKind::Flt
                    | NativeOperatorKind::Fle
                    | NativeOperatorKind::FAdd
                    | NativeOperatorKind::FSub
                    | NativeOperatorKind::FDiv
                    | NativeOperatorKind::FMul => PrimitiveType::Float64,
                    NativeOperatorKind::BEq => PrimitiveType::Bool,
                };

                self.envs
                    .set_type(&left.hir_id.clone(), &arg_t.clone().into());
                self.envs.set_type(&right.hir_id.clone(), &arg_t.into());

                self.visit_native_operator(op);
            }
            ExpressionKind::FunctionCall(fc) => {
                self.visit_expression(&fc.op);

                walk_list!(self, visit_expression, &fc.args);

                self.setup_call(fc, &fc.op.get_hir_id());
            }
            ExpressionKind::Indice(i) => {
                self.visit_expression(&i.op);
                self.visit_expression(&i.value);

                let value_t = self.envs.get_type(&i.value.get_hir_id()).unwrap().clone();

                match self.envs.get_type(&i.op.get_hir_id()).unwrap().clone() {
                    Type::Primitive(PrimitiveType::Array(inner, size)) => {
                        self.envs.set_type(&i.get_hir_id(), &inner);

                        match self.envs.get_type(&i.value.get_hir_id()).unwrap().clone() {
                            Type::Primitive(PrimitiveType::Int64) => {
                                if let ExpressionKind::Lit(literal) = &*i.value.kind {
                                    if literal.as_number() >= size as i64 {
                                        // Deactivated for now
                                        // self.envs.diagnostics.push_error(
                                        //     Diagnostic::new_out_of_bounds(
                                        //         self.envs
                                        //             .spans
                                        //             .get(&i.value.get_hir_id())
                                        //             .unwrap()
                                        //             .clone(),
                                        //         i.value.as_literal().as_number() as u64,
                                        //         size as u64,
                                        //     ),
                                        // )
                                    }
                                }
                            }
                            other => {
                                self.envs
                                    .diagnostics
                                    .push_error(Diagnostic::new_type_conflict(
                                        self.envs.spans.get(&i.value.get_hir_id()).unwrap().clone(),
                                        Type::Primitive(PrimitiveType::Int64),
                                        other.clone(),
                                        Type::Primitive(PrimitiveType::Int64),
                                        other,
                                    ))
                            }
                        }
                    }
                    other => self
                        .envs
                        .diagnostics
                        .push_error(Diagnostic::new_type_conflict(
                            self.envs.spans.get(&i.value.get_hir_id()).unwrap().clone(),
                            Type::Primitive(PrimitiveType::Array(Box::new(value_t.clone()), 0)),
                            other.clone(),
                            Type::Primitive(PrimitiveType::Array(Box::new(value_t), 0)),
                            other,
                        )),
                }
            }
            ExpressionKind::Dot(d) => {
                self.visit_expression(&d.op);
                self.visit_identifier(&d.value);

                match &self.envs.get_type(&d.op.get_hir_id()).unwrap().clone() {
                    t @ Type::Struct(struct_t) => {
                        self.envs.set_type(&d.op.get_hir_id(), t);

                        self.envs
                            .set_type(&d.get_hir_id(), struct_t.defs.get(&d.value.name).unwrap());

                        if let Type::Func(_ft) = &**struct_t.defs.get(&d.value.name).unwrap() {
                            let resolved = self.resolve(&d.value.get_hir_id()).unwrap();

                            self.add_tmp_resolution_to_current_fn(&d.get_hir_id(), &resolved);
                        }
                    }
                    other => {
                        let value_t = self.envs.get_type(&d.value.get_hir_id()).unwrap().clone();

                        self.envs
                            .diagnostics
                            .push_error(Diagnostic::new_type_conflict(
                                self.envs.spans.get(&d.value.get_hir_id()).unwrap().clone(),
                                value_t.clone(),
                                other.clone(),
                                value_t,
                                other.clone(),
                            ))
                    }
                }
            }
        }
    }

    fn visit_literal(&mut self, lit: &Literal) {
        let t = match &lit.kind {
            LiteralKind::Number(_n) => Type::Primitive(PrimitiveType::Int64),
            LiteralKind::Float(_f) => Type::Primitive(PrimitiveType::Float64),
            LiteralKind::String(_s) => Type::Primitive(PrimitiveType::String),
            LiteralKind::Bool(_b) => Type::Primitive(PrimitiveType::Bool),
            LiteralKind::Array(arr) => {
                self.visit_array(arr);

                let inner_t = self.envs.get_type(&arr.get_hir_id()).unwrap();

                Type::Primitive(PrimitiveType::Array(
                    Box::new(inner_t.clone()),
                    arr.values.len(),
                ))
            }
        };

        self.envs.set_type(&lit.hir_id, &t);
    }

    fn visit_array(&mut self, arr: &'a Array) {
        let mut arr = arr.clone();

        let first = arr.values.remove(0);

        self.visit_expression(&first);

        for value in &arr.values {
            self.visit_expression(value);

            self.envs
                .set_type_eq(&value.get_hir_id(), &first.get_hir_id());
        }
    }

    fn visit_for_in(&mut self, for_in: &'a ForIn) {
        self.visit_expression(&for_in.expr);

        self.envs
            .get_type(&for_in.expr.get_hir_id())
            .cloned()
            .and_then(|expr_t| {
                expr_t
                    .is_array()
                    .then(|| expr_t.try_as_primitive_type().unwrap())
                    .and_then(|p| p.try_as_array())
                    .map(|(inner_t, _size)| {
                        self.envs.set_type(&for_in.value.get_hir_id(), &inner_t)
                    })
            });

        self.visit_body(&for_in.body);

        // assert expr to arr type
        // set item to inner type;
    }

    fn visit_identifier_path(&mut self, id: &'a IdentifierPath) {
        self.visit_identifier(id.path.iter().last().unwrap());
    }

    fn visit_identifier(&mut self, id: &Identifier) {
        // We set the type to resolution if any
        if let Some(reso) = self.resolve(&id.hir_id) {
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
            | NativeOperatorKind::Igt
            | NativeOperatorKind::Ige
            | NativeOperatorKind::Ilt
            | NativeOperatorKind::Ile
            | NativeOperatorKind::FEq
            | NativeOperatorKind::Fgt
            | NativeOperatorKind::Fge
            | NativeOperatorKind::Flt
            | NativeOperatorKind::Fle
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

        self.envs.set_type(&op.hir_id, &t.into());
    }
}

pub fn solve(root: &mut Root) -> (BTreeMap<HirId, ResolutionMap<HirId>>, Diagnostics) {
    let diagnostics = Diagnostics::default();

    let infer_state = Envs::new(diagnostics, root.get_hir_spans());

    let mut constraint_ctx = ConstraintContext::new(infer_state, root);

    constraint_ctx.constraint(root);

    let tmp_resolutions = constraint_ctx.tmp_resolutions.clone();

    let envs = constraint_ctx.get_envs();

    root.type_envs = envs.clone();

    (tmp_resolutions, envs.get_diagnostics())
}
