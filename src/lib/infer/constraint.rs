use super::{Constraint, InferState};
use crate::hir::visit::*;
use crate::hir::*;

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
                        if let TopLevelKind::Function(f) = &top.kind {
                            println!("OUAWEJHAWKEALWKJEH");
                            self.state.add_constraint(Constraint::Eq(
                                self.state.get_type_id(op_hir_id).unwrap(),
                                self.state.get_type_id(f.hir_id.clone()).unwrap(),
                            ));
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

    // fn visit_function_decl(&mut self, f: &FunctionDecl) {
    //     // let args = arguments.constrain_vec(ctx);
    //     // let body_type = self.body.constrain(ctx);

    //     // let self_type_id = ctx.get_type_id(self.identity.clone()).unwrap();

    //     // ctx.solve_type(
    //     //     self.identity.clone(),
    //     //     Type::FuncType(FuncType::new((*self.name).clone(), args, body_type)),
    //     // );

    //     // ctx.add_constraint(Constraint::Eq(self_type_id, body_type));

    //     // self_type_id
    //     // self.new_named_annotation((*argument.name).clone(), argument.hir_id.clone());
    // }

    // fn visit_argument_decl(&mut self, argument: &ArgumentDecl) {}
}
