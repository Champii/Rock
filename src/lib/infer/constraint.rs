use std::collections::HashMap;

use crate::{
    ast::{FuncType, PrimitiveType, Type, TypeSignature},
    diagnostics::Diagnostics,
    hir::visit::*,
    hir::*,
    walk_list, walk_map, Envs,
};

#[derive(Debug)]
struct ConstraintContext<'a> {
    hir: &'a Root,
    envs: Envs,
}

impl<'a> ConstraintContext<'a> {
    pub fn new(envs: Envs, hir: &'a Root) -> Self {
        Self { envs, hir }
    }

    pub fn constraint(&mut self, root: &'a Root) {
        let entry_point = root.get_function_by_name("main").unwrap();

        self.envs.set_current_fn((
            entry_point.hir_id.clone(),
            TypeSignature::default().with_ret(Type::int64()),
        ));

        self.visit_function_decl(&entry_point);
    }

    pub fn get_envs(self) -> Envs {
        self.envs
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
                let op_hir_id = fc.op.get_terminal_hir_id();

                if let Some(top_id) = self.hir.resolutions.get_recur(&op_hir_id) {
                    if let Some(reso) = self.hir.arena.get(&top_id) {
                        match reso {
                            HirNode::Prototype(p) => {
                                // let sig_ret_t_id = self
                                //     .state
                                //     .get_or_create_type_id_by_type(&p.signature.ret)
                                //     .unwrap();

                                // let constraint = Constraint::Callable(
                                //     self.state.get_type_id(fc.op.get_terminal_hir_id()).unwrap(),
                                //     sig_ret_t_id.clone(),
                                // );

                                // self.state.add_constraint(Constraint::Eq(
                                //     self.state.get_type_id(fc.hir_id.clone()).unwrap(),
                                //     sig_ret_t_id.clone(),
                                // ));

                                // for (i, arg) in p.signature.args.iter().enumerate() {
                                //     let constraint = Constraint::Eq(
                                //         self.state.get_or_create_type_id_by_type(arg).unwrap(),
                                //         self.state
                                //             .get_type_id(
                                //                 fc.args.get(i).unwrap().get_terminal_hir_id(),
                                //             )
                                //             .unwrap(),
                                //     );

                                //     self.state.add_constraint(constraint);
                                // }

                                // self.state.add_constraint(constraint);
                            }
                            HirNode::FunctionDecl(f) => {
                                let sig = f.signature.apply_partial_types(
                                    &f.arguments
                                        .iter()
                                        .enumerate()
                                        .into_iter()
                                        .map(|(i, arg)| {
                                            self.envs
                                                .get_type(&fc.args.get(i).unwrap().get_hir_id())
                                                .cloned()
                                        })
                                        .collect(),
                                    None,
                                );

                                if self.envs.get_current_fn().0 == top_id {
                                    warn!("Recursion !");

                                    self.envs.set_type(&fc.get_hir_id(), &sig.ret);
                                    self.envs.set_type(&fc.op.get_hir_id(), &sig.to_func_type());

                                    return;
                                }

                                let old_f = self.envs.get_current_fn();

                                self.envs.set_current_fn((top_id, sig.clone()));

                                self.visit_function_decl(f);

                                let new_f_type = self.envs.get_type(&f.hir_id).unwrap().clone();
                                self.envs.set_current_fn(old_f);

                                self.envs.set_type(&fc.get_hir_id(), &sig.ret);
                                self.envs.set_type(&fc.op.get_hir_id(), &new_f_type);
                            }
                            _ => (),
                        }
                    } else {
                        panic!("NO ARENA ITEM FOR HIR={:?}", top_id);
                    }
                } else {
                    panic!("No reso");
                }
            }
        }
    }

    fn visit_literal(&mut self, lit: &Literal) {
        let t = match &lit.kind {
            LiteralKind::Number(_n) => Type::Primitive(PrimitiveType::Int64),
            LiteralKind::Float(_f) => Type::Primitive(PrimitiveType::Float64),
            LiteralKind::String(s) => Type::Primitive(PrimitiveType::String(s.len())),
            LiteralKind::Bool(_b) => Type::Primitive(PrimitiveType::Bool),
        };

        self.envs.set_type(&lit.hir_id, &t);
    }

    fn visit_prototype(&mut self, p: &Prototype) {
        // let args = p
        //     .signature
        //     .args
        //     .iter()
        //     .map(|t| self.state.get_or_create_type_id_by_type(t).unwrap())
        //     .collect();

        // let f = Type::FuncType(FuncType::new(
        //     (*p.name).clone(),
        //     args,
        //     self.state
        //         .get_or_create_type_id_by_type(&p.signature.ret)
        //         .unwrap(),
        // ));

        // self.state.solve_type(p.hir_id.clone(), f);
    }

    fn visit_identifier_path(&mut self, id: &'a IdentifierPath) {
        self.visit_identifier(&id.path.iter().last().unwrap());
    }

    fn visit_identifier(&mut self, id: &Identifier) {
        if let Some(reso) = self.hir.resolutions.get_recur(&id.hir_id) {
            if self.envs.get_type(&reso).is_some() {
                self.envs.set_type_eq(&id.get_hir_id(), &reso);
            } else {
                error!(
                    "UNKNOWN IDENTIFIER RESOLUTION TYPE ID {:?}, reso {:?}",
                    id, reso
                );
            }
        } else {
            error!("No identifier resolution {:?}", id);
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

pub fn solve<'a>(root: &mut Root) -> Diagnostics {
    let mut diagnostics = Diagnostics::default();

    let infer_state = Envs::default();

    let mut constraint_ctx = ConstraintContext::new(infer_state, &root);

    constraint_ctx.constraint(&root);

    let mut envs = constraint_ctx.get_envs();

    root.type_envs = envs;

    // if let Err(diags) = infer_state.solve() {
    //     for diag in diags {
    //         diagnostics.push_error(diag.clone());
    //     }
    // }

    diagnostics
}
