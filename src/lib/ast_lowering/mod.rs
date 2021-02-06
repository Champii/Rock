use std::collections::{BTreeMap, HashMap};

use crate::{
    ast::*,
    hir::{self, BodyId, HirId},
    NodeId,
};

#[derive(Clone, Debug)]
pub struct HirMap {
    map: HashMap<HirId, NodeId>,
    rev_map: HashMap<NodeId, HirId>,
}

impl HirMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            rev_map: HashMap::new(),
        }
    }

    pub fn next_hir_id(&mut self, identity: Identity) -> HirId {
        let hir_id = HirId::next();

        self.add_hir_mapping(hir_id.clone(), identity.node_id);

        hir_id
    }

    pub fn get_hir_id(&self, node_id: NodeId) -> Option<HirId> {
        self.rev_map.get(&node_id).cloned()
    }

    fn add_hir_mapping(&mut self, hir_id: HirId, node_id: NodeId) {
        self.map.insert(hir_id.clone(), node_id);

        self.rev_map.insert(node_id, hir_id.clone());
    }
}

pub struct AstLoweringContext {
    hir_map: HirMap,
    top_levels: BTreeMap<HirId, hir::TopLevel>,
    modules: BTreeMap<HirId, hir::Mod>,
    bodies: BTreeMap<BodyId, hir::Body>,
}

impl AstLoweringContext {
    pub fn new() -> Self {
        Self {
            hir_map: HirMap::new(),
            top_levels: BTreeMap::new(),
            modules: BTreeMap::new(),
            bodies: BTreeMap::new(),
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

        let mut top_level = hir::TopLevel {
            kind: self.lower_top_level_kind(&top_level.kind),
            hir_id: id.clone(),
        };

        let child_id = top_level.get_child_hir();
        top_level.hir_id = child_id.clone();

        self.top_levels.insert(child_id.clone(), top_level);

        return child_id.clone();
    }

    pub fn lower_top_level_kind(&mut self, top_level_kind: &TopLevelKind) -> hir::TopLevelKind {
        match top_level_kind {
            TopLevelKind::Function(f) => hir::TopLevelKind::Function(self.lower_function_decl(&f)),
        }
    }

    pub fn lower_function_decl(&mut self, f: &FunctionDecl) -> hir::FunctionDecl {
        let body_id = BodyId::next();
        let id = self.hir_map.next_hir_id(f.identity.clone());
        let ident = self.lower_identifier(&f.name);

        let body = self.lower_body(&f.body, ident.clone());

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

    pub fn lower_body(&mut self, body: &Body, name: hir::Identifier) -> hir::Body {
        hir::Body {
            name,
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

    pub fn lower_identifier(&mut self, id: &Identifier) -> hir::Identifier {
        let hir_id = self.hir_map.next_hir_id(id.identity.clone());

        hir::Identifier {
            hir_id,
            name: id.name.clone(),
        }
    }
}

pub fn lower_crate(root: &Root) -> hir::Root {
    AstLoweringContext::new().lower_root(root)
}
