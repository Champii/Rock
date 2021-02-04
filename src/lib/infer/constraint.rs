use super::{Constraint, InferState};
use crate::{ast::FuncType, hir::*};
use crate::{ast::Type, hir::visit::*};

#[derive(Debug)]
pub struct ConstraintContext<'a> {
    hir: &'a Root,
    state: InferState,
}

impl<'a> ConstraintContext<'a> {
    pub fn new(state: InferState, hir: &'a Root) -> Self {
        Self { state, hir }
    }

    pub fn constraint(&mut self, root: &'a Root) {
        self.visit_root(root);
    }

    pub fn get_state(self) -> InferState {
        self.state
    }
}

impl<'a> Visitor<'a> for ConstraintContext<'a> {
    fn visit_expression(&mut self, expr: &'a Expression) {
        match &*expr.kind {
            ExpressionKind::Lit(lit) => self.visit_literal(&lit),
            ExpressionKind::Identifier(id) => self.visit_identifier(&id),
            ExpressionKind::FunctionCall(op, args) => {
                // get fn sig
                // add constraint for each args

                self.visit_expression(&op);

                walk_list!(self, visit_expression, args);
                let op_hir_id = op.get_terminal_hir_id();

                // TODO: Use global resolution instead of top_level
                // TODO: Need Arena and a way to fetch any element/item/node
                if let Some(top_id) = self.hir.resolutions.get(op_hir_id.clone()) {
                    if let Some(top) = self.hir.get_top_level(top_id.clone()) {
                        match &top.kind {
                            TopLevelKind::Function(f) => {
                                self.state.add_constraint(Constraint::Eq(
                                    self.state.get_type_id(op_hir_id).unwrap(),
                                    self.state.get_type_id(f.hir_id.clone()).unwrap(),
                                ));

                                let mut i = 0;
                                for arg in &f.arguments {
                                    self.state.add_constraint(Constraint::Eq(
                                        self.state.get_type_id(arg.name.hir_id.clone()).unwrap(),
                                        self.state
                                            .get_type_id(args.get(i).unwrap().get_terminal_hir_id())
                                            .unwrap(),
                                    ));

                                    i += 1;
                                }
                            }
                        }
                    } else {
                        panic!("No top");
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
            let body_type_id = self.state.get_type_id(body_hir_id).unwrap();

            let self_type_id = self.state.get_type_id(f.hir_id.clone()).unwrap();

            self.state.solve_type(
                f.hir_id.clone(),
                Type::FuncType(FuncType::new((*f.name).clone(), args, body_type_id)),
            );

            self.state
                .add_constraint(Constraint::Eq(self_type_id, body_type_id));
        }
    }
}
