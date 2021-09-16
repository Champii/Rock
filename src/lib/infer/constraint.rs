use std::collections::HashMap;

use crate::{
    ast::{FuncType, PrimitiveType, Type},
    diagnostics::Diagnostics,
    hir::visit::*,
    hir::*,
    walk_list, walk_map,
};

use super::{annotate::AnnotateContext, Constraint, InferState};

#[derive(Debug)]
pub struct ConstraintContext<'a> {
    hir: &'a Root,
    state: InferState<'a>,
    current_body: Option<FnBodyId>,
    new_resolutions: HashMap<HirId, HirId>,
}

impl<'a> ConstraintContext<'a> {
    pub fn new(state: InferState<'a>, hir: &'a Root) -> Self {
        Self {
            state,
            hir,
            current_body: None,
            new_resolutions: HashMap::new(),
        }
    }

    pub fn constraint(&mut self, root: &'a Root) {
        self.visit_root(root);
    }

    pub fn get_state(self) -> (InferState<'a>, HashMap<HirId, HirId>) {
        (self.state, self.new_resolutions)
    }
}

impl<'a, 'ar> Visitor<'a> for ConstraintContext<'ar> {
    fn visit_root(&mut self, root: &'a Root) {
        walk_list!(self, visit_top_level, &root.top_levels);

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
                self.state.add_constraint(Constraint::Eq(
                    self.state.get_type_id(p.name.get_hir_id()).unwrap(),
                    self.state.get_type_id(top_level.get_hir_id()).unwrap(),
                ));
                self.visit_prototype(&p);
            }
            TopLevelKind::Function(f) => {
                // self.state.add_constraint(Constraint::Eq(
                //     self.state.get_type_id(f.name.get_hir_id()).unwrap(),
                //     self.state.get_type_id(top_level.get_hir_id()).unwrap(),
                // ));
                self.visit_function_decl(&f);
            }
        };
    }

    fn visit_trait(&mut self, _t: &'a Trait) {}

    fn visit_fn_body(&mut self, fn_body: &'a FnBody) {
        self.current_body = Some(fn_body.id.clone());
        self.visit_body(&fn_body.body);
    }

    fn visit_body(&mut self, body: &'a Body) {
        body.stmts
            .iter()
            .for_each(|stmt| self.visit_statement(&stmt));
    }

    fn visit_assign(&mut self, assign: &'a Assign) {
        self.visit_identifier(&assign.name);
        self.visit_expression(&assign.value);

        self.state.add_constraint(Constraint::Eq(
            self.state.get_type_id(assign.name.hir_id.clone()).unwrap(),
            self.state
                .get_type_id(assign.value.get_terminal_hir_id())
                .unwrap(),
        ));
    }

    fn visit_if(&mut self, r#if: &'a If) {
        self.visit_expression(&r#if.predicat);

        self.visit_body(&r#if.body);

        self.state.add_constraint(Constraint::Eq(
            self.state
                .get_type_id(r#if.body.get_terminal_hir_id())
                .unwrap(),
            self.state.get_type_id(r#if.hir_id.clone()).unwrap(),
        ));

        if let Some(e) = &r#if.else_ {
            match &**e {
                Else::Body(b) => {
                    self.state.add_constraint(Constraint::Eq(
                        self.state
                            .get_type_id(r#if.body.get_terminal_hir_id())
                            .unwrap(),
                        self.state.get_type_id(b.get_terminal_hir_id()).unwrap(),
                    ));

                    self.visit_body(b);
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

                self.state
                    .solve_type(left.hir_id.clone(), Type::Primitive(arg_t.clone()));
                self.state
                    .solve_type(right.hir_id.clone(), Type::Primitive(arg_t));

                self.state.add_constraint(Constraint::Eq(
                    self.state.get_type_id(left.hir_id.clone()).unwrap(),
                    self.state.get_type_id(right.hir_id.clone()).unwrap(),
                ));

                self.visit_native_operator(&op);
            }
            ExpressionKind::FunctionCall(fc) => {
                self.visit_expression(&fc.op);

                walk_list!(self, visit_expression, &fc.args);
                let op_hir_id = fc.op.get_terminal_hir_id();

                // self.state.add_constraint(Constraint::Callable(
                //     self.state.get_type_id(fc.op.get_terminal_hir_id()).unwrap(),
                //     self.state.get_type_id(fc.get_hir_id()).unwrap(),
                // ));

                if let Some(top_id) = self.hir.resolutions.get_recur(&op_hir_id) {
                    if let Some(reso) = self.hir.arena.get(&top_id) {
                        match reso {
                            HirNode::Prototype(p) => {
                                let sig_ret_t_id = self
                                    .state
                                    .get_or_create_type_id_by_type(&p.signature.ret)
                                    .unwrap();

                                let constraint = Constraint::Callable(
                                    self.state.get_type_id(fc.op.get_terminal_hir_id()).unwrap(),
                                    sig_ret_t_id.clone(),
                                );

                                self.state.add_constraint(Constraint::Eq(
                                    self.state.get_type_id(fc.hir_id.clone()).unwrap(),
                                    sig_ret_t_id.clone(),
                                ));

                                for (i, arg) in p.signature.args.iter().enumerate() {
                                    let constraint = Constraint::Eq(
                                        self.state.get_or_create_type_id_by_type(arg).unwrap(),
                                        self.state
                                            .get_type_id(
                                                fc.args.get(i).unwrap().get_terminal_hir_id(),
                                            )
                                            .unwrap(),
                                    );

                                    self.state.add_constraint(constraint);
                                }

                                self.state.add_constraint(constraint);
                            }
                            HirNode::FunctionDecl(f) => {
                                let body = self.hir.get_body(f.body_id.clone()).unwrap();

                                let body_hir_id = body.get_terminal_hir_id();
                                let body_type_id =
                                    self.state.get_type_id(body_hir_id.clone()).unwrap();

                                self.state.add_constraint(Constraint::Eq(
                                    self.state.get_type_id(fc.hir_id.clone()).unwrap(),
                                    body_type_id,
                                ));

                                self.state.add_constraint(Constraint::Callable(
                                    self.state.get_type_id(fc.op.get_terminal_hir_id()).unwrap(),
                                    body_type_id,
                                ));

                                for (i, arg) in f.arguments.iter().enumerate() {
                                    self.state.add_constraint(Constraint::Eq(
                                        self.state.get_type_id(arg.name.hir_id.clone()).unwrap(),
                                        self.state
                                            .get_type_id(
                                                fc.args.get(i).unwrap().get_terminal_hir_id(),
                                            )
                                            .unwrap(),
                                    ));
                                }
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

    fn visit_function_decl(&mut self, f: &FunctionDecl) {
        let args = f
            .arguments
            .iter()
            .map(|arg| self.state.get_type_id(arg.name.hir_id.clone()).unwrap())
            .collect();

        if let Some(body) = self.hir.get_body(f.body_id.clone()) {
            let body_hir_id = body.get_terminal_hir_id();
            let body_type_id = self.state.get_type_id(body_hir_id.clone()).unwrap();

            self.state.add_constraint(Constraint::Callable(
                self.state.get_type_id(f.hir_id.clone()).unwrap(),
                body_type_id,
            ));

            self.state.add_constraint(Constraint::Eq(
                self.state.get_type_id(f.hir_id.clone()).unwrap(),
                self.state.get_type_id(f.name.hir_id.clone()).unwrap(),
            ));

            self.state.solve_type(
                f.hir_id.clone(),
                Type::FuncType(FuncType::new(f.get_name().name, args, body_type_id)),
            );
        }
    }

    fn visit_prototype(&mut self, p: &Prototype) {
        let args = p
            .signature
            .args
            .iter()
            .map(|t| self.state.get_or_create_type_id_by_type(t).unwrap())
            .collect();

        let f = Type::FuncType(FuncType::new(
            (*p.name).clone(),
            args,
            self.state
                .get_or_create_type_id_by_type(&p.signature.ret)
                .unwrap(),
        ));

        self.state.solve_type(p.hir_id.clone(), f);
    }

    fn visit_identifier_path(&mut self, id: &'a IdentifierPath) {
        self.visit_identifier(&id.path.iter().last().unwrap());
    }

    fn visit_identifier(&mut self, id: &Identifier) {
        if let Some(reso) = self.hir.resolutions.get(&id.hir_id) {
            if self.state.get_type_id(reso.clone()).is_none() {
                error!(
                    "UNKNOWN IDENTIFIER RESOLUTION TYPE ID {:?}, reso {:?}",
                    id, reso
                );

                return;
            }

            self.state.add_constraint(Constraint::Eq(
                self.state.get_type_id(id.hir_id.clone()).unwrap(),
                self.state.get_type_id(reso.clone()).unwrap(),
            ));
        } else {
            error!("No identifier resolution {:?}", id);
        }
    }
}

pub fn solve<'a>(mut root: Root) -> (Root, Diagnostics) {
    let mut diagnostics = Diagnostics::default();

    let infer_state = InferState::new(&root);

    let mut annotate_ctx = AnnotateContext::new(infer_state, root.trait_methods.clone());
    annotate_ctx.annotate(&root);

    let mut constraint_ctx = ConstraintContext::new(annotate_ctx.get_state(), &root);
    constraint_ctx.constraint(&root);

    let (mut infer_state, mut _new_resolutions) = constraint_ctx.get_state();

    if let Err(diags) = infer_state.solve() {
        for diag in diags {
            diagnostics.push_error(diag.clone());
        }
    }

    let node_type_ids = infer_state.get_node_types();

    // here we consume infer_state
    let (types, diags) = infer_state.get_types();

    root.node_types = node_type_ids
        .iter()
        .map(|(hir_id, t_id)| (hir_id.clone(), types.get(t_id).unwrap().clone()))
        .collect();

    root.node_type_ids = node_type_ids;
    root.types = types;

    for diag in diags {
        diagnostics.push_error(diag.clone());
    }

    (root, diagnostics)
}
