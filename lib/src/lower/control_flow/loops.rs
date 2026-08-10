use crate::ast;
use crate::hir::*;
use crate::lower::Lowerer;
use crate::types::Type;

impl Lowerer {
    pub(crate) fn lower_loop(&mut self, loop_expr: &ast::Loop) -> HirExpr {
        let span = self.diagnostics.current_span().cloned().unwrap_or_default();
        match loop_expr {
            ast::Loop::While(cond, body) => {
                let condition = self.lower_expression(&cond.expression);
                let _ = self.engine.unify(&condition.ty, &Type::Bool);

                self.scope.push();
                let body = self.lower_block(body);
                self.scope.pop();

                HirExpr {
                    ty: Type::Unit,
                    kind: HirExprKind::While {
                        condition: Box::new(condition),
                        body,
                    },
                    span,
                }
            }
            ast::Loop::For(pattern, iter_expr, body) => {
                let iter = self.lower_expression(iter_expr);
                let (var_name, _) = self.extract_pattern_binding(pattern);

                let elem_ty = if matches!(&iter.kind, HirExprKind::Range(_, _)) {
                    Type::I64
                } else {
                    match self.engine.resolve(&iter.ty) {
                        Type::Array(element, _) | Type::Slice(element) => *element,
                        _ => {
                            let elem_ty = self.engine.fresh_type_var();
                            let slice_ty = Type::Slice(Box::new(elem_ty.clone()));
                            let _ = self.engine.unify(&iter.ty, &slice_ty);
                            elem_ty
                        }
                    }
                };

                self.scope.push();
                let local_id = self.fresh_local_id();
                self.scope
                    .define_local(var_name.clone(), elem_ty, false, local_id);
                let body = self.lower_block(body);
                self.scope.pop();

                HirExpr {
                    ty: Type::Unit,
                    kind: HirExprKind::For {
                        var: var_name,
                        local_id,
                        iter: Box::new(iter),
                        body,
                    },
                    span,
                }
            }
            ast::Loop::Loop(body) => {
                self.scope.push();
                let body = self.lower_block(body);
                self.scope.pop();

                HirExpr {
                    ty: Type::Unit,
                    kind: HirExprKind::Loop(body),
                    span,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{
        Block, Expression, Ident, IdentOrNumber, IdentOrType, IdentifierPath, Literal, LiteralKind,
        Operand, Pattern, PatternKind, PrimaryExpr, SecondaryExpr, Statement, UnaryExpr,
    };
    use crate::hir::{HirExprKind, HirStmt, HirVarRef, HirVarTarget};
    use crate::lexer::Span;
    use crate::lower::Lowerer;

    fn ident(name: &str) -> Ident {
        Ident {
            name: name.to_string(),
            span: Span::default(),
        }
    }

    fn binding_pattern(name: &str) -> Pattern {
        Pattern {
            binding: Some(ident(name)),
            kind: PatternKind::Ident(crate::ast::IdentPattern {
                name: ident(name),
                mut_: false,
            }),
        }
    }

    fn range_expr(start: u64, end: u64) -> Expression {
        Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
            operand: Operand::Literal(Literal {
                kind: LiteralKind::Number(start),
                span: Span::default(),
            }),
            secondaries: Some(vec![SecondaryExpr::DoubleDot(IdentOrNumber::Number(end))]),
            type_annotation: None,
        }))
    }

    fn var_expr(name: &str) -> Expression {
        Expression::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
            operand: Operand::Ident(IdentifierPath {
                path: vec![IdentOrType::Ident(ident(name))],
            }),
            secondaries: None,
            type_annotation: None,
        }))
    }

    #[test]
    fn for_loop_variable_read_uses_loop_local_id() {
        let mut lowerer = Lowerer::new();
        let loop_expr = crate::ast::Loop::For(
            binding_pattern("item"),
            range_expr(0, 2),
            Block {
                statements: vec![Statement::Expression(var_expr("item"))],
            },
        );

        let hir = lowerer.with_test_body_context(|lowerer| lowerer.lower_loop(&loop_expr));

        let HirExprKind::For { local_id, body, .. } = hir.kind else {
            panic!("expected for loop");
        };
        let HirStmt::Expr(expr) = &body.stmts[0] else {
            panic!("expected loop body expression");
        };
        match &expr.kind {
            HirExprKind::ResolvedVar(HirVarRef {
                target: HirVarTarget::Local(id),
                name,
            }) => {
                assert_eq!(name, "item");
                assert_eq!(*id, local_id);
            }
            other => panic!("expected loop local read, got {other:?}"),
        }
    }
}
