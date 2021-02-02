use std::collections::BTreeMap;

use crate::ast::Type;
use crate::hir::hir_id::*;

#[derive(Debug, Clone)]
pub struct Root {
    pub top_levels: BTreeMap<HirId, TopLevel>,
    pub modules: BTreeMap<HirId, Mod>,
    pub bodies: BTreeMap<BodyId, Body>,
}

impl Root {
    pub fn get_top_level(&self, hir_id: HirId) -> Option<&TopLevel> {
        self.top_levels.get(&hir_id)
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
}

#[derive(Debug, Clone)]
pub struct ArgumentDecl {
    // pub hir_id: HirId,
    pub name: Identifier,
    pub t: Option<Type>,
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
    // If(If),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct If {
    pub predicat: Expression,
    pub body: Expression,
    pub else_: Option<Box<Else>>,
}

#[derive(Debug, Clone)]
pub enum Else {
    // If(If),
// Body(Body),
}

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
    pub fn new_identifier(id: Identifier) -> Self {
        Self {
            kind: Box::new(ExpressionKind::Identifier(id)),
        }
    }
    pub fn new_function_call(op: Expression, args: Vec<Expression>) -> Self {
        Self {
            kind: Box::new(ExpressionKind::FunctionCall(op, args)),
        }
    }

    pub fn get_terminal_hir_id(&self) -> HirId {
        match &*self.kind {
            ExpressionKind::Lit(l) => l.hir_id.clone(),
            ExpressionKind::Identifier(i) => i.hir_id.clone(),
            ExpressionKind::FunctionCall(op, args) => op.get_terminal_hir_id(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExpressionKind {
    // Binop(Expression, Operator, Expression),
    Lit(Literal),
    Identifier(Identifier),
    FunctionCall(Expression, Vec<Expression>),
}

// #[derive(Debug, Clone)]
// pub enum Operator {
//     Add,
//     Sub,
//     Sum,
//     Div,
//     Mod,

//     Less,
//     LessOrEqual,
//     More,
//     MoreOrEqual,

//     EqualEqual,
//     DashEqual,
// }

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
