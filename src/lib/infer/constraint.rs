use std::collections::HashMap;

use crate::ast::PrimitiveType;

use super::{Constraint, InferState};
use crate::walk_list;
use crate::{ast::FuncType, hir::*};
use crate::{ast::Type, hir::visit::*};

#[derive(Debug)]
pub struct ConstraintContext<'a> {
    hir: &'a Root,
    state: InferState,
    current_body: Option<FnBodyId>,
    new_resolutions: HashMap<HirId, HirId>,
}

impl<'a> ConstraintContext<'a> {
    pub fn new(state: InferState, hir: &'a Root) -> Self {
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

    pub fn get_state(self) -> (InferState, HashMap<HirId, HirId>) {
        (self.state, self.new_resolutions)
    }
}

impl<'a> Visitor<'a> for ConstraintContext<'a> {
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

                // FIXME: Code smell
                // TODO: Use global resolution instead of top_level
                // TODO: Need Arena and a way to fetch any element/item/node
                if let Some(top_id) = self.hir.resolutions.get_recur(&op_hir_id) {
                    if let Some(top) = self.hir.get_top_level(top_id.clone()) {
                        match &top.kind {
                            TopLevelKind::Prototype(p) => {
                                let constraint = Constraint::Callable(
                                    self.state.get_type_id(fc.hir_id.clone()).unwrap(),
                                    self.state
                                        .get_or_create_type_id_by_type(&p.signature.ret)
                                        .unwrap(),
                                );

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
                            TopLevelKind::Function(f) => {
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
                        }
                    } else {
                        // Trait solving

                        // FIXME: Apply to list of types
                        // FIXME: Type needs to be solved in order to be applied. There is a dependency loop here
                        if let Err(_) = self.state.solve() {
                            //Ignoring this diagnostic, as it will be produced again later
                        }

                        let first_resolved_type = fc
                            .args
                            .iter()
                            .filter_map(|arg| {
                                self.state
                                    .get_type(self.state.get_type_id(arg.get_terminal_hir_id())?)
                            })
                            .nth(0);

                        if let Some(applied_type) = first_resolved_type {
                            // FIXME: Copy-paste of the code above
                            if let Some(f) = self.hir.match_trait_method(
                                fc.op.as_identifier().clone().name,
                                &applied_type,
                            ) {
                                let body = self.hir.get_body(f.body_id.clone()).unwrap();

                                let body_hir_id = body.get_terminal_hir_id();
                                let body_type_id =
                                    self.state.get_type_id(body_hir_id.clone()).unwrap();

                                self.state.add_constraint(Constraint::Callable(
                                    self.state.get_type_id(fc.op.get_terminal_hir_id()).unwrap(),
                                    body_type_id,
                                ));

                                self.state.add_constraint(Constraint::Eq(
                                    self.state.get_type_id(fc.hir_id.clone()).unwrap(),
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

                                let r#trait = self
                                    .hir
                                    .get_trait_by_method(fc.op.as_identifier().clone().name)
                                    .unwrap();

                                self.new_resolutions
                                    .insert(fc.op.get_terminal_hir_id(), f.name.hir_id.clone());
                                self.new_resolutions
                                    .insert(fc.hir_id.clone(), f.name.hir_id.clone());

                                self.state.trait_call_to_mangle.insert(
                                    fc.hir_id.clone(),
                                    vec![r#trait.name.get_name(), applied_type.get_name()],
                                );
                            } else {
                                // println!("NO TRAIT METHOD RESOLUTION");
                                self.state.add_constraint(Constraint::Callable(
                                    self.state.get_type_id(fc.op.get_terminal_hir_id()).unwrap(),
                                    self.state.get_type_id(fc.hir_id.clone()).unwrap(),
                                ));
                                self.state.add_constraint(Constraint::Callable(
                                    self.state.get_type_id(top_id.clone()).unwrap(),
                                    self.state.get_type_id(fc.hir_id.clone()).unwrap(),
                                ));
                            }
                        } else {
                            self.state.solve_type(
                                fc.op.get_terminal_hir_id(),
                                Type::FuncType(FuncType::new(
                                    fc.op.as_identifier().name,
                                    fc.args
                                        .iter()
                                        .map(|arg| {
                                            self.state
                                                .get_type_id(arg.get_terminal_hir_id())
                                                .unwrap()
                                        })
                                        .collect::<Vec<_>>(),
                                    self.state.get_type_id(fc.hir_id.clone()).unwrap(),
                                )),
                            );
                            self.state.solve_type(
                                top_id.clone(),
                                Type::FuncType(FuncType::new(
                                    fc.op.as_identifier().name,
                                    fc.args
                                        .iter()
                                        .map(|arg| {
                                            self.state
                                                .get_type_id(arg.get_terminal_hir_id())
                                                .unwrap()
                                        })
                                        .collect::<Vec<_>>(),
                                    self.state.get_type_id(fc.hir_id.clone()).unwrap(),
                                )),
                            );
                        }
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
        if let Some(reso) = self.hir.resolutions.get_recur(&id.hir_id) {
            // self.state
            //     .new_named_annotation(id.name.clone(), reso.clone());

            if self.state.get_type_id(reso.clone()).is_none() {
                warn!("UNKNOWN IDENTIFIER RESOLUTION TYPE ID {:?}", id);

                self.state.new_type_id(reso.clone());
            }

            self.state.add_constraint(Constraint::Eq(
                self.state.get_type_id(id.hir_id.clone()).unwrap(),
                self.state.get_type_id(reso.clone()).unwrap(),
            ));
        } else {
            warn!("No identifier resolution {:?}", id);
            // self.state
            //     .new_named_annotation(id.name.clone(), id.hir_id.clone());
        }
    }
}
