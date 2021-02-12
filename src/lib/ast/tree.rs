use crate::ast::identity::Identity;
use crate::{ast::helper::*, NodeId};

use crate::ast::resolve::ResolutionMap;

#[derive(Debug, Clone)]
pub struct Root {
    pub r#mod: Mod,
    pub resolutions: ResolutionMap<NodeId>,
    // pub identity: Identity,
}

#[derive(Debug, Clone)]
pub struct Path {
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub name: Identifier,
}

#[derive(Debug, Clone)]
pub struct Mod {
    // pub name: Path,
    pub top_levels: Vec<TopLevel>,
    pub identity: Identity,
}

#[derive(Debug, Clone)]
pub struct TopLevel {
    pub kind: TopLevelKind,
    pub identity: Identity,
}

#[derive(Debug, Clone)]
pub enum TopLevelKind {
    Function(FunctionDecl),
    Mod(Identifier, Mod),
    // Use(Path),
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: Identifier,
    pub arguments: Vec<ArgumentDecl>,
    pub body: Body,
    pub identity: Identity,
}

generate_has_name!(FunctionDecl);

#[derive(Debug, Clone)]
pub struct IdentifierPath {
    pub path: Vec<Identifier>,
}

#[derive(Debug, Clone)]
pub struct Identifier {
    pub name: String,
    pub identity: Identity,
}

impl std::ops::Deref for Identifier {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

generate_has_name!(Identifier);

pub type ArgumentsDecl = Vec<ArgumentDecl>;

#[derive(Debug, Clone)]
pub struct ArgumentDecl {
    pub name: String,
    pub identity: Identity,
}

#[derive(Debug, Clone)]
pub struct Body {
    pub stmt: Statement,
    // pub identity: Identity,
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: Box<StatementKind>,
    // pub identity: Identity,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    If(If),
    // For(For),
    Expression(Expression),
    // Assignation(Assignation),
}

#[derive(Debug, Clone)]
pub struct If {
    pub predicat: Expression,
    pub body: Body,
    pub else_: Option<Box<Else>>,
}

#[derive(Debug, Clone)]
pub struct Expression {
    pub kind: ExpressionKind,
    // pub identity: Identity,
}

impl Expression {
    pub fn is_literal(&self) -> bool {
        match &self.kind {
            ExpressionKind::UnaryExpr(unary) => unary.is_literal(),
            _ => false,
        }
    }

    pub fn is_identifier(&self) -> bool {
        match &self.kind {
            ExpressionKind::UnaryExpr(unary) => unary.is_identifier(),
            _ => false,
        }
    }

    // pub fn get_identifier(&self) -> Option<String> {
    //     match &self.kind {
    //         ExpressionKind::UnaryExpr(unary) => unary.get_identifier(),
    //         _ => None,
    //     }
    // }
}

#[derive(Debug, Clone)]
pub enum ExpressionKind {
    BinopExpr(UnaryExpr, Operator, Box<Expression>),
    UnaryExpr(UnaryExpr),
}

#[derive(Debug, Clone)]
pub enum Else {
    If(If),
    Body(Body),
}

#[derive(Debug, Clone)]
pub enum UnaryExpr {
    PrimaryExpr(PrimaryExpr),
    UnaryExpr(Operator, Box<UnaryExpr>),
}

impl UnaryExpr {
    pub fn is_literal(&self) -> bool {
        match self {
            UnaryExpr::PrimaryExpr(p) => match p {
                PrimaryExpr::PrimaryExpr(operand, _) => {
                    matches!(&operand.kind, OperandKind::Literal(_))
                }
            },
            _ => false,
        }
    }

    pub fn is_identifier(&self) -> bool {
        match self {
            UnaryExpr::PrimaryExpr(p) => match p {
                PrimaryExpr::PrimaryExpr(operand, _) => {
                    matches!(&operand.kind, OperandKind::Identifier(_))
                }
            },
            _ => false,
        }
    }

    // pub fn get_identifier(&self) -> Option<String> {
    //     match self {
    //         UnaryExpr::PrimaryExpr(p) => match p {
    //             PrimaryExpr::PrimaryExpr(operand, vec) => match &operand.kind {
    //                 OperandKind::Identifier(i) => {
    //                     if vec.len() == 0 {
    //                         Some(i.name.clone())
    //                     } else {
    //                         None
    //                     }
    //                 }
    //                 _ => None,
    //             },
    //         },
    //         _ => None,
    //     }
    // }
}

#[derive(Debug, Clone)]
pub enum Operator {
    Add,
    Sub,
    Sum,
    Div,
    Mod,

    Less,
    LessOrEqual,
    More,
    MoreOrEqual,

    EqualEqual,
    DashEqual,
}

#[derive(Debug, Clone)]
pub enum PrimaryExpr {
    PrimaryExpr(Operand, Vec<SecondaryExpr>),
}

impl PrimaryExpr {
    pub fn has_secondaries(&self) -> bool {
        match self {
            PrimaryExpr::PrimaryExpr(_, vec) => !vec.is_empty(),
        }
    }

    // pub fn get_identifier(&self) -> Option<String> {
    //     match self {
    //         PrimaryExpr::PrimaryExpr(op, _) => {
    //             if let OperandKind::Identifier(ident) = &op.kind {
    //                 Some(ident.name.clone())
    //             } else {
    //                 None
    //             }
    //         }
    //     }
    // }
}

#[derive(Debug, Clone)]
pub struct Operand {
    pub kind: OperandKind,
    // pub identity: Identity,
}

#[derive(Debug, Clone)]
pub enum OperandKind {
    Literal(Literal),
    Identifier(IdentifierPath),
    // ClassInstance(ClassInstance),
    // Array(Array),
    Expression(Box<Expression>), // parenthesis
}

#[derive(Debug, Clone)]
pub enum SecondaryExpr {
    // Selector(Selector), // . Identifier  // u8 is the attribute index in struct // option<Type> is the class type if needed // RealFullName
    Arguments(Vec<Argument>), // (Expr, Expr, ...)
                              // Index(Box<Expression>), // [Expr]
}

#[derive(Debug, Clone)]
pub struct Literal {
    pub kind: LiteralKind,
    pub identity: Identity,
}

#[derive(Debug, Clone)]
pub enum LiteralKind {
    Number(i64),
    String(String),
    Bool(u64),
}

pub type Arguments = Vec<Argument>;

#[derive(Debug, Clone)]
pub struct Argument {
    pub arg: Expression,
    // pub identity: Identity,
}
