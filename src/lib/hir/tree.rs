use std::collections::BTreeMap;

use crate::{ast::resolve::ResolutionMap, hir::hir_id::*, TypeId};
use crate::{ast::Type, ast_lowering::HirMap};

#[derive(Debug, Clone)]
pub struct Root {
    pub hir_map: HirMap,
    pub resolutions: ResolutionMap<HirId>,
    pub node_types: BTreeMap<HirId, TypeId>,
    pub types: BTreeMap<TypeId, Type>,
    pub top_levels: BTreeMap<HirId, TopLevel>,
    pub modules: BTreeMap<HirId, Mod>,
    pub bodies: BTreeMap<BodyId, Body>,
}

impl Root {
    pub fn get_top_level(&self, hir_id: HirId) -> Option<&TopLevel> {
        self.top_levels.get(&hir_id)
    }

    pub fn get_body(&self, body_id: BodyId) -> Option<&Body> {
        self.bodies.get(&body_id)
    }

    pub fn get_type(&self, hir_id: HirId) -> Option<Type> {
        let t_id = self.node_types.get(&hir_id)?;

        self.types.get(&t_id).cloned()
    }
}

#[derive(Debug, Clone)]
pub struct Mod {
    pub top_levels: Vec<HirId>,
    pub hir_id: HirId,
}

#[derive(Debug, Clone)]
pub struct TopLevel {
    pub kind: TopLevelKind,
    pub hir_id: HirId,
}

impl TopLevel {
    pub fn get_child_hir(&self) -> HirId {
        match &self.kind {
            TopLevelKind::Function(f) => f.hir_id.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TopLevelKind {
    Function(FunctionDecl),
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: Identifier,
    pub arguments: Vec<ArgumentDecl>,
    pub ret: Type,
    pub body_id: BodyId,
    pub hir_id: HirId,
}

#[derive(Debug, Clone)]
pub struct ArgumentDecl {
    pub name: Identifier,
}

#[derive(Debug, Clone)]
pub struct IdentifierPath {
    pub path: Vec<Identifier>,
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub hir_id: HirId,
    pub name: String,
}

impl std::ops::Deref for Identifier {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

#[derive(Debug, Clone)]
pub struct Body {
    pub id: BodyId,
    pub name: Identifier,
    pub stmt: Statement,
}

impl Body {
    pub fn get_terminal_hir_id(&self) -> HirId {
        self.stmt.get_terminal_hir_id()
    }
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: Box<StatementKind>,
}

impl Statement {
    pub fn get_terminal_hir_id(&self) -> HirId {
        match &*self.kind {
            StatementKind::Expression(e) => e.get_terminal_hir_id(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct If {
    pub predicat: Expression,
    pub body: Expression,
    pub else_: Option<Box<Else>>,
}

#[derive(Debug, Clone)]
pub enum Else {}

#[derive(Debug, Clone)]
pub struct Expression {
    pub kind: Box<ExpressionKind>,
}

impl Expression {
    pub fn new_literal(lit: Literal) -> Self {
        Self {
            kind: Box::new(ExpressionKind::Lit(lit)),
        }
    }
    pub fn new_identifier_path(id: IdentifierPath) -> Self {
        Self {
            kind: Box::new(ExpressionKind::Identifier(id)),
        }
    }
    pub fn new_function_call(f: FunctionCall) -> Self {
        Self {
            kind: Box::new(ExpressionKind::FunctionCall(f)),
        }
    }
    pub fn new_native_operation(op: NativeOperator, left: Identifier, right: Identifier) -> Self {
        Self {
            kind: Box::new(ExpressionKind::NativeOperation(op, left, right)),
        }
    }

    pub fn get_terminal_hir_id(&self) -> HirId {
        match &*self.kind {
            ExpressionKind::Lit(l) => l.hir_id.clone(),
            ExpressionKind::Identifier(i) => i.path.iter().last().unwrap().hir_id.clone(),
            ExpressionKind::FunctionCall(fc) => fc.hir_id.clone(),
            ExpressionKind::NativeOperation(op, _left, _right) => op.hir_id.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExpressionKind {
    Lit(Literal),
    Identifier(IdentifierPath),
    FunctionCall(FunctionCall),
    NativeOperation(NativeOperator, Identifier, Identifier),
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub hir_id: HirId,
    pub op: Expression,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub struct Literal {
    pub hir_id: HirId,
    pub kind: LiteralKind,
}

#[derive(Debug, Clone)]
pub enum LiteralKind {
    Number(i64),
    String(String),
    Bool(u64),
}

#[derive(Debug, Clone)]
pub struct NativeOperator {
    pub hir_id: HirId,
    pub kind: NativeOperatorKind,
}

#[derive(Debug, Clone)]
pub enum NativeOperatorKind {
    Add,
    Sub,
    Mul,
    Div,
}
