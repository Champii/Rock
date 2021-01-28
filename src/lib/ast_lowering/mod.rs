use std::collections::BTreeMap;

use crate::ast::*;
use crate::hir;
use crate::hir::{BodyId, HirId};

pub struct AstLoweringContext {
    top_levels: BTreeMap<HirId, hir::TopLevel>,
    modules: BTreeMap<HirId, hir::Mod>,
    bodies: BTreeMap<BodyId, hir::Body>,
}
impl AstLoweringContext {
    pub fn new() -> Self {
        Self {
            top_levels: BTreeMap::new(),
            modules: BTreeMap::new(),
            bodies: BTreeMap::new(),
        }
    }

    pub fn lower_root(&mut self, root: &Root) -> hir::Root {
        // self.modules.insert(self.lower_mod(&root.r#mod));
        self.lower_mod(&root.r#mod);

        hir::Root {
            top_levels: self.top_levels.clone(),
            modules: self.modules.clone(),
            bodies: self.bodies.clone(),
        }
    }

    pub fn lower_mod(&mut self, r#mod: &Mod) -> hir::HirId {
        let id = HirId::next();

        let r#mod = hir::Mod {
            hir_id: id.clone(),
            top_levels: r#mod
                .top_levels
                .iter()
                .map(|t| self.lower_top_level(&t))
                .collect(),
        };

        self.modules.insert(id.clone(), r#mod);

        id
    }

    pub fn lower_top_level(&mut self, top_level: &TopLevel) -> hir::HirId {
        let id = HirId::next();

        let top_level = hir::TopLevel {
            kind: self.lower_top_level_kind(&top_level.kind),
            hir_id: id.clone(),
        };

        self.top_levels.insert(id.clone(), top_level);

        return id.clone();
    }

    pub fn lower_top_level_kind(&mut self, top_level_kind: &TopLevelKind) -> hir::TopLevelKind {
        match top_level_kind {
            TopLevelKind::Function(f) => hir::TopLevelKind::Function(self.lower_function_decl(&f)),
        }
    }

    pub fn lower_function_decl(&mut self, f: &FunctionDecl) -> hir::FunctionDecl {
        let body_id = BodyId::next();

        let body = self.lower_body(&f.body);

        self.bodies.insert(body_id.clone(), body);

        hir::FunctionDecl {
            arguments: f
                .arguments
                .iter()
                .map(|arg| self.lower_argument_decl(&arg))
                .collect(),
            ret: Type::Undefined(0),
            body_id,
        }
    }

    pub fn lower_argument_decl(&mut self, argument: &ArgumentDecl) -> Type {
        // TODO: create and assign placeholder type id
        Type::Undefined(0)
    }

    pub fn lower_body(&mut self, body: &Body) -> hir::Body {
        hir::Body {
            stmt: self.lower_statement(&body.stmt),
        }
    }

    pub fn lower_statement(&mut self, stmt: &Statement) -> hir::Statement {
        hir::Statement {
            kind: match &*stmt.kind {
                // StatementKind::If(i) => hir::StatementKind::If(self.lower_if(i)),
                StatementKind::Expression(e) => {
                    Box::new(hir::StatementKind::Expression(self.lower_expression(&e)))
                }
                _ => unimplemented!(),
            },
        }
    }

    pub fn lower_expression(&mut self, expr: &Expression) -> hir::Expression {
        match &expr.kind {
            ExpressionKind::UnaryExpr(unary) => self.lower_unary(&unary),
            _ => unimplemented!(),
        }
    }

    pub fn lower_unary(&mut self, unary: &UnaryExpr) -> hir::Expression {
        match &unary {
            UnaryExpr::PrimaryExpr(primary) => self.lower_primary(&primary),
            _ => unimplemented!(),
        }
    }

    pub fn lower_primary(&mut self, primary: &PrimaryExpr) -> hir::Expression {
        match &primary {
            PrimaryExpr::PrimaryExpr(op, secs) => {
                if secs.len() == 0 {
                    return self.lower_operand(&op);
                }

                hir::Expression::new_function_call(
                    self.lower_operand(&op),
                    secs.iter()
                        .map(|sec| self.lower_secondary(&sec))
                        .flatten()
                        .collect(),
                )
            }
        }
    }

    pub fn lower_operand(&mut self, operand: &Operand) -> hir::Expression {
        match &operand.kind {
            OperandKind::Literal(l) => hir::Expression::new_literal(self.lower_literal(&l)),
            OperandKind::Identifier(i) => {
                hir::Expression::new_identifier(self.lower_identifier(&i))
            }
            OperandKind::Expression(e) => self.lower_expression(&**e),
        }
    }

    pub fn lower_secondary(&mut self, secondary: &SecondaryExpr) -> Vec<hir::Expression> {
        match secondary {
            SecondaryExpr::Arguments(args) => args
                .iter()
                .map(|arg| self.lower_expression(&arg.arg))
                .collect(),
        }
    }

    pub fn lower_literal(&mut self, lit: &Literal) -> hir::Literal {
        hir::Literal {
            kind: match &lit.kind {
                LiteralKind::Number(n) => hir::LiteralKind::Number(*n),
                LiteralKind::String(s) => hir::LiteralKind::String(s.clone()),
                LiteralKind::Bool(b) => hir::LiteralKind::Bool(*b),
            },
        }
    }

    pub fn lower_identifier(&mut self, id: &Identifier) -> hir::Identifier {
        hir::Identifier {
            name: id.name.clone(),
        }
    }
}

pub fn lower_crate(root: &Root) -> hir::Root {
    AstLoweringContext::new().lower_root(root)
}
