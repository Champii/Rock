use std::collections::HashMap;

use crate::ast::identity::Identity;
use crate::NodeId;

use crate::ast::resolve::ResolutionMap;
use crate::generate_has_name;
use crate::helpers::*;

#[derive(Debug, Clone)]
pub struct Root {
    pub r#mod: Mod,
    pub resolutions: ResolutionMap<NodeId>,
    pub operators_list: HashMap<String, u8>,
}

#[derive(Debug, Clone)]
pub struct Mod {
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
    Prototype(Prototype),
    Function(FunctionDecl),
    Mod(Identifier, Mod),
    Use(Use),
    Infix(Identifier, u8),
}

#[derive(Debug, Clone)]
pub struct Prototype {
    pub name: Identifier,
    pub arguments: Vec<ArgumentDecl>,
    pub identity: Identity,
}

#[derive(Debug, Clone)]
pub struct Use {
    pub path: IdentifierPath,
    pub identity: Identity,
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: Identifier,
    pub arguments: Vec<ArgumentDecl>,
    pub body: Body,
    pub identity: Identity,
}

generate_has_name!(FunctionDecl);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct IdentifierPath {
    pub path: Vec<Identifier>,
}

impl IdentifierPath {
    pub fn new_root() -> Self {
        Self {
            path: vec![Identifier {
                name: "root".to_string(),
                identity: Identity::new_placeholder(),
            }],
        }
    }

    pub fn parent(&self) -> Self {
        let mut parent = self.clone();

        if parent.path.len() > 1 {
            parent.path.pop();
        }

        parent
    }

    pub fn child(&self, name: Identifier) -> Self {
        let mut child = self.clone();

        child.path.push(name);

        child
    }

    pub fn last_segment(&self) -> Identifier {
        self.path.iter().last().unwrap().clone()
    }

    pub fn last_segment_ref(&self) -> &Identifier {
        self.path.iter().last().unwrap()
    }

    pub fn prepend_mod(&self, path: IdentifierPath) -> Self {
        let mut path = path.clone();

        path.path.extend::<_>(self.path.clone());

        path
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Identifier {
    pub name: String,
    pub identity: Identity,
}

impl PartialEq for Identifier {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl std::hash::Hash for Identifier {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
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
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: Box<StatementKind>,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    Expression(Expression),
    If(If),
}

#[derive(Debug, Clone)]
pub struct If {
    pub identity: Identity,
    pub predicat: Expression,
    pub body: Body,
    pub else_: Option<Box<Else>>,
}

#[derive(Debug, Clone)]
pub enum Else {
    If(If),
    Body(Body),
}

#[derive(Debug, Clone)]
pub struct Expression {
    pub kind: ExpressionKind,
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

    pub fn is_binop(&self) -> bool {
        match &self.kind {
            ExpressionKind::BinopExpr(_, _, _) => true,
            _ => false,
        }
    }

    pub fn from_unary(unary: &UnaryExpr) -> Expression {
        Expression {
            kind: ExpressionKind::UnaryExpr(unary.clone()),
        }
    }

    pub fn create_2_args_func_call(op: Operand, arg1: UnaryExpr, arg2: UnaryExpr) -> Expression {
        Expression {
            kind: ExpressionKind::UnaryExpr(UnaryExpr::PrimaryExpr(PrimaryExpr {
                identity: Identity::new_placeholder(),
                op,
                secondaries: Some(vec![SecondaryExpr::Arguments(vec![
                    Argument { arg: arg1 },
                    Argument { arg: arg2 },
                ])]),
            })),
        }
    }
}
#[derive(Debug, Clone)]
pub enum ExpressionKind {
    BinopExpr(UnaryExpr, Operator, Box<Expression>),
    UnaryExpr(UnaryExpr),
    NativeOperation(NativeOperator, Identifier, Identifier),
    Return(Box<Expression>),
}

#[derive(Debug, Clone)]
pub enum UnaryExpr {
    PrimaryExpr(PrimaryExpr),
    UnaryExpr(Operator, Box<UnaryExpr>),
}

impl UnaryExpr {
    pub fn is_literal(&self) -> bool {
        match self {
            UnaryExpr::PrimaryExpr(p) => matches!(&p.op.kind, OperandKind::Literal(_)),
            _ => false,
        }
    }

    pub fn is_identifier(&self) -> bool {
        match self {
            UnaryExpr::PrimaryExpr(p) => matches!(&p.op.kind, OperandKind::Identifier(_)),
            _ => false,
        }
    }

    pub fn create_2_args_func_call(op: Operand, arg1: UnaryExpr, arg2: UnaryExpr) -> UnaryExpr {
        UnaryExpr::PrimaryExpr(PrimaryExpr {
            identity: Identity::new_placeholder(),
            op,
            secondaries: Some(vec![SecondaryExpr::Arguments(vec![
                Argument { arg: arg1 },
                Argument { arg: arg2 },
            ])]),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Operator(pub Identifier);

#[derive(Debug, Clone)]
pub struct PrimaryExpr {
    pub identity: Identity,
    pub op: Operand,
    pub secondaries: Option<Vec<SecondaryExpr>>,
}

impl PrimaryExpr {
    pub fn has_secondaries(&self) -> bool {
        self.secondaries.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct Operand {
    pub kind: OperandKind,
}

impl Operand {
    pub fn from_identifier(id: &Identifier) -> Self {
        Self {
            kind: OperandKind::Identifier(IdentifierPath {
                path: vec![id.clone()],
            }),
        }
    }

    pub fn is_literal(&self) -> bool {
        matches!(&self.kind, OperandKind::Literal(_))
    }

    pub fn is_identifier(&self) -> bool {
        matches!(&self.kind, OperandKind::Identifier(_))
    }
}

#[derive(Debug, Clone)]
pub enum OperandKind {
    Literal(Literal),
    Identifier(IdentifierPath),
    Expression(Box<Expression>), // parenthesis
}

#[derive(Debug, Clone)]
pub enum SecondaryExpr {
    Arguments(Vec<Argument>),
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
    pub arg: UnaryExpr,
}

#[derive(Debug, Clone)]
pub struct NativeOperator {
    pub kind: NativeOperatorKind,
    pub identity: Identity,
}

#[derive(Debug, Clone)]
pub enum NativeOperatorKind {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    GT,
    GE,
    LT,
    LE,
}
