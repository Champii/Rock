use super::{Constraint, InferState};
use crate::walk_list;
use crate::{ast::FuncType, hir::*};
use crate::{ast::Type, hir::visit::*};

#[derive(Debug)]
pub struct ConstraintContext<'a> {
    hir: &'a Root,
    state: InferState,
    current_body: Option<BodyId>,
}

impl<'a> ConstraintContext<'a> {
    pub fn new(state: InferState, hir: &'a Root) -> Self {
        Self {
            state,
            hir,
            current_body: None,
        }
    }

    pub fn constraint(&mut self, root: &'a Root) {
        self.visit_root(root);
    }

    pub fn get_state(self) -> InferState {
        self.state
    }
}

impl<'a> Visitor<'a> for ConstraintContext<'a> {
    fn visit_body(&mut self, body: &'a Body) {
        self.current_body = Some(body.id.clone());
        self.visit_statement(&body.stmt);
    }

    fn visit_expression(&mut self, expr: &'a Expression) {
        match &*expr.kind {
            ExpressionKind::Lit(lit) => self.visit_literal(&lit),
            ExpressionKind::Identifier(id) => self.visit_identifier_path(&id),
            ExpressionKind::NativeOperation(op, left, right) => {
                self.state.add_constraint(Constraint::Eq(
                    self.state.get_type_id(op.hir_id.clone()).unwrap(),
                    self.state.get_type_id(left.hir_id.clone()).unwrap(),
                ));

                self.state.add_constraint(Constraint::Eq(
                    self.state.get_type_id(left.hir_id.clone()).unwrap(),
                    self.state.get_type_id(right.hir_id.clone()).unwrap(),
                ));
            }
            ExpressionKind::FunctionCall(fc) => {
                self.visit_expression(&fc.op);

                walk_list!(self, visit_expression, &fc.args);
                let op_hir_id = fc.op.get_terminal_hir_id();

                // TODO: Use global resolution instead of top_level
                // TODO: Need Arena and a way to fetch any element/item/node
                if let Some(top_id) = self.hir.resolutions.get(op_hir_id.clone()) {
                    if let Some(top) = self.hir.get_top_level(top_id.clone()) {
                        match &top.kind {
                            TopLevelKind::Function(f) => {
                                let body = self.hir.get_body(f.body_id.clone()).unwrap();

                                let body_hir_id = body.get_terminal_hir_id();
                                let body_type_id =
                                    self.state.get_type_id(body_hir_id.clone()).unwrap();

                                self.state.add_constraint(Constraint::Callable(
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
                            }
                        }
                    } else {
                        self.state.add_constraint(Constraint::Callable(
                            self.state.get_type_id(fc.hir_id.clone()).unwrap(),
                            self.state.get_type_id(top_id).unwrap(),
                        ));
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

            self.state.solve_type(
                f.hir_id.clone(),
                Type::FuncType(FuncType::new((*f.name).clone(), args, body_type_id)),
            );
        }
    }
}
