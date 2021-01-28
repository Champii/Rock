use std::collections::BTreeMap;

use crate::ast::Type;
use crate::hir::hir_id::*;

#[derive(Debug, Clone)]
pub struct Root {
    pub top_levels: BTreeMap<HirId, TopLevel>,
    pub modules: BTreeMap<HirId, Mod>,
    pub bodies: BTreeMap<BodyId, Body>,
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
    pub arguments: Vec<Type>,
    pub ret: Type,
    pub body_id: BodyId,
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub name: String,
}

impl std::ops::Deref for Identifier {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

pub type ArgumentsDecl = Vec<ArgumentDecl>;

#[derive(Debug, Clone, Default)]
pub struct ArgumentDecl {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Body {
    pub stmt: Statement,
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: Box<StatementKind>,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    If(If),
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
    If(If),
    Body(Body),
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
    pub kind: LiteralKind,
}

#[derive(Debug, Clone)]
pub enum LiteralKind {
    Number(i64),
    String(String),
    Bool(u64),
}
