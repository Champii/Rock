use std::collections::{BTreeMap, HashMap};

use crate::{
    ast::{resolve::ResolutionMap, Type, TypeSignature},
    ast_lowering::HirMap,
    hir::hir_id::*,
    parser::Span,
    Envs, NodeId, TypeId,
};

use super::{arena::Arena, HasHirId};

#[derive(Debug, Default)]
pub struct Root {
    pub arena: Arena,
    pub hir_map: HirMap,
    pub resolutions: ResolutionMap<HirId>,
    pub node_type_ids: BTreeMap<HirId, TypeId>,
    pub type_envs: Envs,
    pub types: BTreeMap<TypeId, Type>,
    pub node_types: BTreeMap<HirId, Type>,
    pub traits: HashMap<Type, Trait>, // TraitHirId => (Trait, TypeId => Impl)
    pub trait_methods: HashMap<String, HashMap<TypeSignature, FunctionDecl>>,
    pub top_levels: Vec<TopLevel>,
    pub bodies: BTreeMap<FnBodyId, FnBody>,
    pub spans: HashMap<NodeId, Span>,
}

impl Root {
    pub fn get_top_level(&self, hir_id: HirId) -> Option<&TopLevel> {
        self.top_levels
            .iter()
            .find(|top| top.get_terminal_hir_id() == hir_id)
    }

    pub fn get_trait_by_method(&self, ident: String) -> Option<Trait> {
        self.traits
            .iter()
            .find(|(_, r#trait)| {
                r#trait
                    .defs
                    .iter()
                    .find(|proto| *proto.name == ident)
                    .is_some()
            })
            .map(|(_, r#trait)| r#trait.clone())
    }

    pub fn get_trait_method(
        &self,
        ident: String,
        applied_type: &TypeSignature,
    ) -> Option<FunctionDecl> {
        self.trait_methods.get(&ident)?.get(applied_type).cloned()
    }

    pub fn match_trait_method(&self, ident: String, applied_type: &Type) -> Option<FunctionDecl> {
        let map = self.trait_methods.get(&ident)?;

        map.iter()
            .find(|(sig, _)| sig.args[0] == *applied_type)
            .map(|(_, fn_decl)| fn_decl.clone())
    }

    pub fn get_function_by_name(&self, name: &str) -> Option<FunctionDecl> {
        self.top_levels
            .iter()
            .find(|top| match &top.kind {
                TopLevelKind::Function(f) => (*f.name) == name,
                _ => false,
            })
            .map(|top| match &top.kind {
                TopLevelKind::Function(f) => f.clone(),
                _ => unimplemented!(),
            })
    }

    pub fn get_body(&self, body_id: &FnBodyId) -> Option<&FnBody> {
        self.bodies.get(body_id)
    }

    pub fn get_function_by_hir_id(&self, hir_id: &HirId) -> Option<&FunctionDecl> {
        self.top_levels
            .iter()
            .find(|top| top.get_hir_id() == *hir_id)
            .and_then(|top| {
                if let TopLevelKind::Function(f) = &top.kind {
                    Some(f)
                } else {
                    None
                }
            })
    }

    pub fn get_type(&self, hir_id: HirId) -> Option<Type> {
        self.type_envs.get_type(&hir_id).cloned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
    pub top_levels: Vec<HirId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trait {
    pub name: Type,
    pub types: Vec<Type>,
    // pub impls: HashMap<Identifier, HashMap<Type, FunctionDecl>>,
    pub defs: Vec<Prototype>,
}

#[derive(Debug, Clone)]
pub struct Impl {
    pub name: Type,
    pub types: Vec<Type>,
    pub defs: Vec<FunctionDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopLevel {
    pub kind: TopLevelKind,
    // pub hir_id: HirId,
}

impl TopLevel {
    pub fn get_terminal_hir_id(&self) -> HirId {
        match &self.kind {
            TopLevelKind::Prototype(p) => p.hir_id.clone(),
            TopLevelKind::Function(f) => f.hir_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TopLevelKind {
    Function(FunctionDecl),
    Prototype(Prototype),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prototype {
    pub name: Identifier,
    pub signature: TypeSignature,
    pub hir_id: HirId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDecl {
    pub name: Identifier,
    pub mangled_name: Option<Identifier>,
    pub arguments: Vec<ArgumentDecl>,
    pub body_id: FnBodyId,
    pub hir_id: HirId,
    pub signature: TypeSignature,
}

impl FunctionDecl {
    pub fn mangle(&mut self, prefixes: Vec<String>) {
        if self.name.name == "main" {
            return;
        }

        self.mangled_name = Some(Identifier {
            name: format!("{}_{}", self.name.name, prefixes.join("_")),
            hir_id: self.name.hir_id.clone(),
        });
    }
    pub fn get_name(&self) -> Identifier {
        match &self.mangled_name {
            Some(name) => name.clone(),
            None => self.name.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentDecl {
    pub name: Identifier,
}

impl ArgumentDecl {
    pub fn get_terminal_hir_id(&self) -> HirId {
        self.name.get_hir_id()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifierPath {
    pub path: Vec<Identifier>,
}

impl IdentifierPath {
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

    pub fn get_terminal_hir_id(&self) -> HirId {
        self.last_segment().get_hir_id()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FnBody {
    pub id: FnBodyId,
    pub fn_id: HirId,
    pub name: Identifier,
    pub mangled_name: Option<Identifier>,
    pub body: Body,
}

impl FnBody {
    pub fn get_terminal_hir_id(&self) -> HirId {
        self.body.get_hir_id()
    }

    pub fn mangle(&mut self, prefixes: &[String]) {
        self.mangled_name = Some(Identifier {
            name: format!("{}_{}", self.name.name, prefixes.join("_")),
            hir_id: self.name.hir_id.clone(),
        });
    }

    pub fn get_name(&self) -> Identifier {
        match &self.mangled_name {
            Some(name) => name.clone(),
            None => self.name.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body {
    pub stmts: Vec<Statement>,
}

impl Body {
    pub fn get_terminal_hir_id(&self) -> HirId {
        self.stmts.iter().last().unwrap().get_hir_id()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    pub kind: Box<StatementKind>,
}

impl Statement {
    pub fn get_terminal_hir_id(&self) -> HirId {
        match &*self.kind {
            StatementKind::Expression(e) => e.get_hir_id(),
            StatementKind::Assign(a) => a.get_hir_id(),
            StatementKind::If(e) => e.get_hir_id(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatementKind {
    Expression(Expression),
    Assign(Assign),
    If(If),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assign {
    // pub hir_id: HirId,
    pub name: Identifier,
    pub value: Expression,
}

impl Assign {
    pub fn get_terminal_hir_id(&self) -> HirId {
        self.name.get_hir_id()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct If {
    pub hir_id: HirId,
    pub predicat: Expression,
    pub body: Body,
    pub else_: Option<Box<Else>>,
}

impl If {
    pub fn get_terminal_hir_id(&self) -> HirId {
        self.hir_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Else {
    If(If),
    Body(Body),
}

impl Else {
    pub fn get_terminal_hir_id(&self) -> HirId {
        match self {
            Else::If(i) => i.get_hir_id(),
            Else::Body(b) => b.get_hir_id(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            ExpressionKind::Lit(l) => l.get_hir_id(),
            ExpressionKind::Identifier(i) => i.get_hir_id(),
            ExpressionKind::FunctionCall(fc) => fc.get_hir_id(),
            ExpressionKind::NativeOperation(op, _left, _right) => op.get_hir_id(),
            ExpressionKind::Return(expr) => expr.get_hir_id(),
        }
    }

    pub fn as_identifier(&self) -> Identifier {
        if let ExpressionKind::Identifier(i) = &*self.kind {
            i.last_segment().clone()
        } else {
            panic!("Not an identifier");
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExpressionKind {
    Lit(Literal),
    Identifier(IdentifierPath),
    FunctionCall(FunctionCall),
    NativeOperation(NativeOperator, Identifier, Identifier),
    Return(Expression),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub hir_id: HirId,
    pub op: Expression,
    pub args: Vec<Expression>,
}

impl FunctionCall {
    // pub fn get_mangled_name(&self, prefixes: Vec<String>) -> Option<String> {
    //     match &*self.op.kind {
    //         ExpressionKind::Identifier(id) => {
    //             let identifier = id.path.iter().last().unwrap();

    //             Some(prefixes.join("_") + "_" + &identifier.name)
    //         }
    //         _ => None, // FIXME: recurse on '(expr)' parenthesis expression
    //     }
    // }

    pub fn mangle(&mut self, prefixes: Vec<String>) {
        match &mut *self.op.kind {
            ExpressionKind::Identifier(id) => {
                let identifier = id.path.iter_mut().last().unwrap();

                identifier.name = format!("{}_{}", identifier.name, &prefixes.join("_"));
                // identifier.name = prefixes.join("_") + "_" + &identifier.name;
            }
            _ => (), // FIXME: recurse on '(expr)' parenthesis expression
        }
    }

    pub fn to_type_signature(&self, env: &BTreeMap<HirId, Type>) -> TypeSignature {
        TypeSignature::from_args_nb(self.args.len()).apply_partial_types(
            &self
                .args
                .iter()
                .map(|arg| env.get(&arg.get_hir_id()).cloned())
                .collect(),
            env.get(&self.hir_id).cloned(),
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Literal {
    pub hir_id: HirId,
    pub kind: LiteralKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiteralKind {
    Number(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeOperator {
    pub hir_id: HirId,
    pub kind: NativeOperatorKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    FLE,
    FGT,
    FGE,
    FLT,
    BEq,
}

impl std::fmt::Display for NativeOperatorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
