use std::collections::{BTreeMap, HashMap};

use crate::{
    ast::*,
    hir::{self, BodyId, HirId},
};

use super::{hir_map::HirMap, InfixDesugar};

pub struct AstLoweringContext {
    hir_map: HirMap,
    top_levels: BTreeMap<HirId, hir::TopLevel>,
    modules: BTreeMap<HirId, hir::Mod>,
    bodies: BTreeMap<BodyId, hir::Body>,
    operators_list: HashMap<String, u8>,
}

impl AstLoweringContext {
    pub fn new(operators_list: HashMap<String, u8>) -> Self {
        Self {
            hir_map: HirMap::new(),
            top_levels: BTreeMap::new(),
            modules: BTreeMap::new(),
            bodies: BTreeMap::new(),
            operators_list,
        }
    }

    pub fn lower_root(&mut self, root: &Root) -> hir::Root {
        self.lower_mod(&root.r#mod);

        hir::Root {
            hir_map: self.hir_map.clone(),
            resolutions: root.resolutions.lower_resolution_map(&self.hir_map),
            node_types: BTreeMap::new(),
            types: BTreeMap::new(),
            top_levels: self.top_levels.clone(),
            modules: self.modules.clone(),
            bodies: self.bodies.clone(),
        }
    }

    pub fn lower_mod(&mut self, r#mod: &Mod) -> hir::HirId {
        let id = self.hir_map.next_hir_id(r#mod.identity.clone());

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
        let id = self.hir_map.next_hir_id(top_level.identity.clone());

        match &top_level.kind {
            TopLevelKind::Infix(_, _) => id,
            TopLevelKind::Function(f) => {
                let mut top_level = hir::TopLevel {
                    kind: hir::TopLevelKind::Function(self.lower_function_decl(&f)),
                    hir_id: id,
                };

                let child_id = top_level.get_child_hir();
                top_level.hir_id = child_id.clone();

                self.top_levels.insert(child_id.clone(), top_level);
                child_id
            }
            TopLevelKind::Mod(_name, mod_) => self.lower_mod(&mod_),
        }
    }

    pub fn lower_function_decl(&mut self, f: &FunctionDecl) -> hir::FunctionDecl {
        let body_id = BodyId::next();
        let id = self.hir_map.next_hir_id(f.identity.clone());
        let ident = self.lower_identifier(&f.name);

        let body = self.lower_body(&f.body, ident.clone(), body_id.clone());

        self.bodies.insert(body_id.clone(), body);

        hir::FunctionDecl {
            name: ident,
            arguments: f
                .arguments
                .iter()
                .map(|arg| self.lower_argument_decl(&arg))
                .collect(),
            ret: Type::Undefined(0),
            body_id,
            hir_id: id,
        }
    }

    pub fn lower_argument_decl(&mut self, argument: &ArgumentDecl) -> hir::ArgumentDecl {
        let id = self.hir_map.next_hir_id(argument.identity.clone());

        hir::ArgumentDecl {
            name: hir::Identifier {
                hir_id: id,
                name: argument.name.clone(),
            },
        }
    }

    pub fn lower_body(&mut self, body: &Body, name: hir::Identifier, body_id: BodyId) -> hir::Body {
        hir::Body {
            id: body_id,
            name,
            stmt: self.lower_statement(&body.stmt),
        }
    }

    pub fn lower_statement(&mut self, stmt: &Statement) -> hir::Statement {
        hir::Statement {
            kind: match &*stmt.kind {
                StatementKind::Expression(e) => {
                    Box::new(hir::StatementKind::Expression(self.lower_expression(&e)))
                }
            },
        }
    }

    pub fn lower_expression(&mut self, expr: &Expression) -> hir::Expression {
        match &expr.kind {
            ExpressionKind::UnaryExpr(unary) => self.lower_unary(&unary),
            ExpressionKind::NativeOperation(op, left, right) => {
                self.lower_native_operation(&op, &left, &right)
            }
            ExpressionKind::BinopExpr(_unary, _op, _expr22) => {
                let mut infix = InfixDesugar::new(self.operators_list.clone());

                self.lower_expression(&infix.desugar(expr))
            }
        }
    }

    pub fn lower_unary(&mut self, unary: &UnaryExpr) -> hir::Expression {
        match &unary {
            UnaryExpr::PrimaryExpr(primary) => self.lower_primary(&primary),
            _ => unimplemented!(),
        }
    }

    pub fn lower_primary(&mut self, primary: &PrimaryExpr) -> hir::Expression {
        if primary.secondaries.is_none() {
            return self.lower_operand(&primary.op);
        }

        hir::Expression::new_function_call(hir::FunctionCall {
            hir_id: self.hir_map.next_hir_id(primary.identity.clone()),
            op: self.lower_operand(&primary.op),
            args: primary
                .secondaries
                .clone()
                .unwrap()
                .iter()
                .map(|sec| self.lower_secondary(&sec))
                .flatten() // FIX: This is bad, we mix secondaries with arguments and we flatten.
                .collect(),
        })
    }

    pub fn lower_operand(&mut self, operand: &Operand) -> hir::Expression {
        match &operand.kind {
            OperandKind::Literal(l) => hir::Expression::new_literal(self.lower_literal(&l)),
            OperandKind::Identifier(i) => {
                hir::Expression::new_identifier_path(self.lower_identifier_path(&i))
            }
            OperandKind::Expression(e) => self.lower_expression(&**e),
        }
    }

    pub fn lower_secondary(&mut self, secondary: &SecondaryExpr) -> Vec<hir::Expression> {
        match secondary {
            SecondaryExpr::Arguments(args) => {
                args.iter().map(|arg| self.lower_unary(&arg.arg)).collect()
            }
        }
    }

    pub fn lower_literal(&mut self, lit: &Literal) -> hir::Literal {
        let hir_id = self.hir_map.next_hir_id(lit.identity.clone());

        hir::Literal {
            hir_id,
            kind: match &lit.kind {
                LiteralKind::Number(n) => hir::LiteralKind::Number(*n),
                LiteralKind::String(s) => hir::LiteralKind::String(s.clone()),
                LiteralKind::Bool(b) => hir::LiteralKind::Bool(*b),
            },
        }
    }

    pub fn lower_identifier_path(&mut self, path: &IdentifierPath) -> hir::IdentifierPath {
        hir::IdentifierPath {
            path: path.path.iter().map(|i| self.lower_identifier(i)).collect(),
        }
    }

    pub fn lower_identifier(&mut self, id: &Identifier) -> hir::Identifier {
        let hir_id = self.hir_map.next_hir_id(id.identity.clone());

        hir::Identifier {
            hir_id,
            name: id.name.clone(),
        }
    }

    pub fn lower_native_operation(
        &mut self,
        op: &NativeOperator,
        left: &Identifier,
        right: &Identifier,
    ) -> hir::Expression {
        hir::Expression::new_native_operation(
            self.lower_native_operator(op),
            self.lower_identifier(left),
            self.lower_identifier(right),
        )
    }

    pub fn lower_native_operator(&mut self, op: &NativeOperator) -> hir::NativeOperator {
        let hir_id = self.hir_map.next_hir_id(op.identity.clone());

        let kind = match op.kind {
            NativeOperatorKind::Add => hir::NativeOperatorKind::Add,
            NativeOperatorKind::Sub => hir::NativeOperatorKind::Sub,
            NativeOperatorKind::Mul => hir::NativeOperatorKind::Mul,
            NativeOperatorKind::Div => hir::NativeOperatorKind::Div,
        };

        hir::NativeOperator { hir_id, kind }
    }
}
