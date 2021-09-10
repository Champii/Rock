use std::collections::HashMap;

use crate::{ast::identity::Identity, parser::Token};
use crate::{parser::Span, NodeId};

use crate::ast::resolve::ResolutionMap;
use crate::generate_has_name;
use crate::helpers::*;

use super::{Type, TypeSignature};

#[derive(Debug, Clone)]
pub struct Root {
    pub r#mod: Mod,
    pub resolutions: ResolutionMap<NodeId>,
    pub operators_list: HashMap<String, u8>,
    pub unused: Vec<NodeId>,
    pub spans: HashMap<NodeId, Span>,
}

#[derive(Debug, Clone)]
pub struct Mod {
    pub tokens: Vec<Token>,
    pub top_levels: Vec<TopLevel>,
    pub identity: Identity,
}

impl Mod {
    pub fn filter_unused_top_levels(&mut self, unused: Vec<NodeId>) {
        let mut unused_trait_method_names = vec![];

        self.top_levels = self
            .top_levels
            .iter_mut()
            .filter_map(|top_level| {
                match &mut top_level.kind {
                    TopLevelKind::Function(f) => {
                        if unused.contains(&f.identity.node_id) {
                            warn!("Unused function {:?}", f.name);

                            return None;
                        }
                    }
                    TopLevelKind::Trait(t) => {
                        let mut defs = vec![];

                        for f in &t.defs {
                            if unused.contains(&f.identity.node_id) {
                                unused_trait_method_names.push(f.name.clone());

                                warn!("Unused trait method {:?}", f.name);
                            } else {
                                defs.push(f.clone());
                            }
                        }

                        if defs.is_empty() {
                            return None;
                        }

                        let mut t2 = t.clone();
                        t2.defs = defs.clone();

                        return Some(TopLevel {
                            kind: TopLevelKind::Trait(t2),
                            ..top_level.clone()
                        });
                    }
                    TopLevelKind::Impl(i) => {
                        let mut defs = vec![];

                        for f in &i.defs {
                            if unused_trait_method_names.contains(&f.name) {
                                warn!("Unused impl method {:?}", f.name);
                            } else {
                                defs.push(f.clone());
                            }
                        }

                        if defs.is_empty() {
                            return None;
                        }

                        let mut i2 = i.clone();
                        i2.defs = defs.clone();

                        return Some(TopLevel {
                            kind: TopLevelKind::Impl(i2),
                            ..top_level.clone()
                        });
                    }
                    TopLevelKind::Mod(id, m) => {
                        m.filter_unused_top_levels(unused.clone());

                        return Some(TopLevel {
                            kind: TopLevelKind::Mod(id.clone(), m.clone()),
                            ..top_level.clone()
                        });
                    }
                    _ => (),
                };
                Some(top_level.clone())
            })
            .collect();
    }
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
    Trait(Trait),
    Impl(Impl),
    Mod(Identifier, Mod),
    Use(Use),
    Infix(Identifier, u8),
}

#[derive(Debug, Clone)]
pub struct Trait {
    pub name: Type,
    pub types: Vec<Type>,
    pub defs: Vec<Prototype>,
}

#[derive(Debug, Clone)]
pub struct Impl {
    pub name: Type,
    pub types: Vec<Type>,
    pub defs: Vec<FunctionDecl>,
}

#[derive(Debug, Clone)]
pub struct Prototype {
    pub name: Identifier,
    pub signature: TypeSignature,
    pub identity: Identity,
}

impl Prototype {
    pub fn mangle(&mut self, prefix: String) {
        self.name.name = prefix + "_" + &self.name.name;
    }
}

#[derive(Debug, Clone)]
pub struct Use {
    pub path: IdentifierPath,
    pub identity: Identity,
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: Identifier,
    // pub mangled_name: Option<Identifier>,
    pub arguments: Vec<ArgumentDecl>,
    pub body: Body,
    pub identity: Identity,
}

impl FunctionDecl {
    pub fn mangle(&mut self, prefixes: &[String]) {
        self.name.name = prefixes.join("_") + "_" + &self.name.name;
    }
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

    pub fn resolve_supers(&mut self) {
        let to_remove = self
            .path
            .iter()
            .enumerate()
            .filter_map(|(i, name)| {
                if name.name == "super".to_string() {
                    Some(i)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut to_remove_total = vec![];

        for id in to_remove {
            to_remove_total.extend(vec![id - 1, id]);
        }

        self.path = self
            .path
            .iter()
            .enumerate()
            .filter_map(|(i, name)| {
                if to_remove_total.contains(&i) {
                    None
                } else {
                    Some(name.clone())
                }
            })
            .collect::<Vec<_>>();
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
    pub stmts: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: Box<StatementKind>,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    Expression(Expression),
    Assign(Assign),
    If(If),
}

#[derive(Debug, Clone)]
pub struct Assign {
    // pub identity: Identity,
    pub name: Identifier,
    pub value: Expression,
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

impl OperandKind {
    pub fn to_identifier_path(&self) -> IdentifierPath {
        if let OperandKind::Identifier(id) = self {
            id.clone()
        } else {
            panic!("Not an identifier path")
        }
    }
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
    Float(f64),
    String(String),
    Bool(bool),
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
    IAdd,
    ISub,
    IMul,
    IDiv,
    FAdd,
    FSub,
    FMul,
    FDiv,
    IEq,
    IGT,
    IGE,
    ILT,
    ILE,
    FEq,
    FGT,
    FGE,
    FLT,
    FLE,
    BEq,
}
