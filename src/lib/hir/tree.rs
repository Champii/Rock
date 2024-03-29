use std::collections::{BTreeMap, HashMap};

use crate::{
    ast::NodeId,
    ast_lowering::HirMap,
    hir::hir_id::*,
    infer::{trait_solver::TraitSolver, Envs},
    parser::span::Span,
    resolver::ResolutionMap,
    ty::{FuncType, StructType, Type},
};

use super::{arena::Arena, hir_printer, HasHirId, HirNode};

#[derive(Debug, Default)]
pub struct Root {
    pub arena: Arena,
    pub hir_map: HirMap,
    pub resolutions: ResolutionMap<HirId>,
    pub type_envs: Envs,
    pub node_types: BTreeMap<HirId, Type>,
    pub traits: HashMap<Type, Trait>,
    pub trait_methods: BTreeMap<String, HashMap<FuncType, FunctionDecl>>,
    pub struct_methods: BTreeMap<HirId, HashMap<FuncType, FunctionDecl>>,
    pub trait_solver: TraitSolver,
    pub top_levels: Vec<TopLevel>,
    pub bodies: BTreeMap<FnBodyId, FnBody>,
    pub spans: HashMap<NodeId, Span>,
    pub structs: HashMap<String, StructDecl>,
    pub signatures: HashMap<HirId, HirId>, // FnHirId -> SignatureHirId
}

impl Root {
    pub fn get_top_level(&self, hir_id: HirId) -> Option<&TopLevel> {
        self.top_levels
            .iter()
            .find(|top| top.get_terminal_hir_id() == hir_id)
    }

    #[allow(dead_code)]
    pub fn get_trait_by_method(&self, ident: String) -> Option<Trait> {
        self.traits
            .iter()
            .find(|(_, r#trait)| r#trait.defs.iter().any(|proto| proto.name.name == ident))
            .map(|(_, r#trait)| r#trait.clone())
    }

    pub fn get_trait_method(&self, ident: String, applied_type: &FuncType) -> Option<FunctionDecl> {
        self.trait_methods
            .get(&ident)?
            .get(applied_type)
            .cloned()
            .or_else(|| {
                self.trait_methods
                    .get(&ident)?
                    .iter()
                    .find(|(sig, _)| sig.arguments == applied_type.arguments)
                    .map(|(_, f)| f.clone())
            })
    }

    #[allow(dead_code)]
    pub fn match_trait_method(&self, ident: String, applied_type: &Type) -> Option<FunctionDecl> {
        let map = self.trait_methods.get(&ident)?;

        map.iter()
            .find(|(sig, _)| sig.arguments[0] == *applied_type)
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

    pub fn get_function_signature_by_name(&self, name: &str) -> Option<FuncType> {
        let fn_decl = self.get_function_by_name(name)?;

        let sig_hir_id = self.signatures.get(&fn_decl.hir_id)?;

        let arena_node = self.arena.get(sig_hir_id)?;

        if let HirNode::Prototype(p) = arena_node {
            Some(p.signature.clone())
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn get_function_by_mangled_name(&self, name: &str) -> Option<FunctionDecl> {
        self.top_levels
            .iter()
            .find(|top| match &top.kind {
                TopLevelKind::Function(f) => {
                    if let Some(n) = &f.mangled_name {
                        **n == name
                    } else {
                        false
                    }
                }
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

    #[allow(dead_code)]
    pub fn get_type(&self, hir_id: HirId) -> Option<Type> {
        self.type_envs.get_type(&hir_id).cloned()
    }

    pub fn get_hir_spans(&self) -> HashMap<HirId, Span> {
        self.spans
            .iter()
            .filter_map(|(node_id, span)| Some((self.hir_map.get_hir_id(*node_id)?, span.clone())))
            .collect()
    }

    pub fn print(&self) {
        hir_printer::print(self);
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
    pub defs: Vec<Prototype>,
}

#[derive(Debug, Clone)]
pub struct Impl {
    pub name: Type,
    pub types: Vec<Type>,
    pub defs: Vec<FunctionDecl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDecl {
    pub name: Identifier,
    pub defs: Vec<Prototype>,
}

impl StructDecl {
    pub fn to_type(&self) -> Type {
        Type::Struct(StructType {
            name: self.name.name.clone(),
            fields_order: self
                .defs
                .iter()
                .map(|proto| proto.name.name.clone())
                .collect(),
            defs: self
                .defs
                .iter()
                .map(|proto| {
                    if proto.signature.arguments.is_empty() {
                        (proto.name.name.clone(), proto.signature.ret.clone())
                    } else {
                        (
                            proto.name.name.clone(),
                            Box::new(Type::Func(proto.signature.clone())),
                        )
                    }
                })
                .collect(),
        })
    }

    pub fn get_terminal_hir_id(&self) -> HirId {
        self.name.hir_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructCtor {
    pub name: Identifier,
    pub defs: BTreeMap<Identifier, Expression>,
}

impl StructCtor {
    pub fn get_terminal_hir_id(&self) -> HirId {
        self.name.hir_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopLevel {
    pub kind: TopLevelKind,
}

impl TopLevel {
    pub fn get_terminal_hir_id(&self) -> HirId {
        match &self.kind {
            TopLevelKind::Extern(p) => p.hir_id.clone(),
            TopLevelKind::Function(f) => f.hir_id.clone(),
            TopLevelKind::Signature(s) => s.hir_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TopLevelKind {
    Function(FunctionDecl),
    Extern(Prototype),
    Signature(Prototype),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prototype {
    pub name: Identifier,
    pub signature: FuncType,
    pub hir_id: HirId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDecl {
    pub name: Identifier,
    pub mangled_name: Option<Identifier>,
    pub arguments: Vec<ArgumentDecl>,
    pub body_id: FnBodyId,
    pub hir_id: HirId,
    pub signature: FuncType,
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
    pub fn last_segment(&self) -> Identifier {
        self.path.iter().last().unwrap().clone()
    }

    pub fn get_terminal_hir_id(&self) -> HirId {
        self.last_segment().get_hir_id()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
            StatementKind::For(f) => f.get_hir_id(),
        }
    }

    pub fn is_return(&self) -> bool {
        match &*self.kind {
            StatementKind::Expression(expr) => expr.is_return(),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatementKind {
    Expression(Expression),
    Assign(Assign),
    If(IfChain),
    For(For),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum For {
    In(ForIn),
    While(While),
}

impl For {
    pub fn get_terminal_hir_id(&self) -> HirId {
        match self {
            For::In(i) => i.get_hir_id(),
            For::While(w) => w.get_hir_id(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct While {
    pub predicat: Expression,
    pub body: Body,
}

impl While {
    pub fn get_terminal_hir_id(&self) -> HirId {
        self.body.get_hir_id()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForIn {
    pub value: Identifier,
    pub expr: Expression,
    pub body: Body,
}

impl ForIn {
    pub fn get_terminal_hir_id(&self) -> HirId {
        self.body.get_hir_id()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssignLeftSide {
    Identifier(Identifier),
    Indice(Indice),
    Dot(Dot),
}

impl AssignLeftSide {
    pub fn get_terminal_hir_id(&self) -> HirId {
        match &self {
            AssignLeftSide::Indice(e) => e.get_hir_id(),
            AssignLeftSide::Identifier(a) => a.get_hir_id(),
            AssignLeftSide::Dot(a) => a.get_hir_id(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assign {
    pub name: AssignLeftSide,
    pub value: Expression,
    pub is_let: bool,
}

impl Assign {
    pub fn get_terminal_hir_id(&self) -> HirId {
        self.name.get_hir_id()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfChain {
    pub ifs: Vec<If>,
    pub else_body: Option<Body>,
}

impl IfChain {
    pub fn get_terminal_hir_id(&self) -> HirId {
        self.ifs.iter().last().unwrap().get_hir_id()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct If {
    pub hir_id: HirId,
    pub predicat: Expression,
    pub body: Body,
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
    pub fn new_struct_ctor(s: StructCtor) -> Self {
        Self {
            kind: Box::new(ExpressionKind::StructCtor(s)),
        }
    }
    pub fn new_indice(f: Indice) -> Self {
        Self {
            kind: Box::new(ExpressionKind::Indice(f)),
        }
    }
    pub fn new_native_operation(op: NativeOperator, left: Identifier, right: Identifier) -> Self {
        Self {
            kind: Box::new(ExpressionKind::NativeOperation(op, left, right)),
        }
    }
    pub fn new_dot(dot: Dot) -> Self {
        Self {
            kind: Box::new(ExpressionKind::Dot(dot)),
        }
    }
    pub fn new_return(ret: Expression) -> Self {
        Self {
            kind: Box::new(ExpressionKind::Return(ret)),
        }
    }

    pub fn get_terminal_hir_id(&self) -> HirId {
        match &*self.kind {
            ExpressionKind::Lit(l) => l.get_hir_id(),
            ExpressionKind::Identifier(i) => i.get_hir_id(),
            ExpressionKind::FunctionCall(fc) => fc.get_hir_id(),
            ExpressionKind::StructCtor(s) => s.get_hir_id(),
            ExpressionKind::Indice(i) => i.get_hir_id(),
            ExpressionKind::Dot(d) => d.get_hir_id(),
            ExpressionKind::NativeOperation(op, _left, _right) => op.get_hir_id(),
            ExpressionKind::Return(expr) => expr.get_hir_id(),
        }
    }

    pub fn get_op_terminal_hir_id(&self) -> HirId {
        match &*self.kind {
            ExpressionKind::Lit(l) => l.get_hir_id(),
            ExpressionKind::Identifier(i) => i.get_hir_id(),
            ExpressionKind::FunctionCall(fc) => fc.get_hir_id(),
            ExpressionKind::StructCtor(s) => s.get_hir_id(),
            ExpressionKind::Indice(i) => i.get_hir_id(),
            ExpressionKind::Dot(d) => d.get_op_terminal_hir_id(),
            ExpressionKind::NativeOperation(op, _left, _right) => op.get_hir_id(),
            ExpressionKind::Return(expr) => expr.get_hir_id(),
        }
    }

    pub fn as_identifier(&self) -> Identifier {
        if let ExpressionKind::Identifier(i) = &*self.kind {
            i.last_segment()
        } else if let ExpressionKind::Dot(d) = &*self.kind {
            d.value.clone()
        } else {
            panic!("Not an identifier");
        }
    }

    #[allow(dead_code)]
    pub fn as_literal(&self) -> Literal {
        if let ExpressionKind::Lit(l) = &*self.kind {
            l.clone()
        } else {
            panic!("Not a literal");
        }
    }

    pub fn as_dot(&self) -> Dot {
        if let ExpressionKind::Dot(l) = &*self.kind {
            l.clone()
        } else {
            panic!("Not a dot");
        }
    }

    pub fn is_return(&self) -> bool {
        if let ExpressionKind::Return(_) = &*self.kind {
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExpressionKind {
    Lit(Literal),
    Identifier(IdentifierPath),
    FunctionCall(FunctionCall),
    StructCtor(StructCtor),
    Indice(Indice),
    Dot(Dot),
    NativeOperation(NativeOperator, Identifier, Identifier),
    Return(Expression),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dot {
    pub hir_id: HirId,
    pub op: Expression,
    pub value: Identifier,
}

impl Dot {
    pub fn get_op_terminal_hir_id(&self) -> HirId {
        match &*self.op.kind {
            ExpressionKind::Dot(d) => d.get_op_terminal_hir_id(),
            _ => self.value.get_hir_id(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Indice {
    pub hir_id: HirId,
    pub op: Expression,
    pub value: Expression,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub hir_id: HirId,
    pub op: Expression,
    pub args: Vec<Expression>,
}

impl FunctionCall {
    pub fn new(hir_id: HirId, op: Expression, args: Vec<Expression>) -> Self {
        Self { hir_id, op, args }
    }
    pub fn mangle(&mut self, prefixes: Vec<String>) {
        if let ExpressionKind::Identifier(id) = &mut *self.op.kind {
            let identifier = id.path.iter_mut().last().unwrap();

            identifier.name = format!("{}_{}", identifier.name, &prefixes.join("_"));
            // _ => unimplemented!("Need to recurse on expr {:#?}", self), // FIXME: recurse on '(expr)' parenthesis expression
        }
    }

    pub fn to_func_type(&self, env: &BTreeMap<HirId, Type>) -> FuncType {
        FuncType::from_args_nb(self.args.len()).apply_partial_types(
            &self
                .args
                .iter()
                .map(|arg| env.get(&arg.get_hir_id()).cloned())
                .collect::<Vec<_>>(),
            env.get(&self.hir_id).cloned(),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Literal {
    pub hir_id: HirId,
    pub kind: LiteralKind,
}

impl Literal {
    pub fn as_number(&self) -> i64 {
        if let LiteralKind::Number(n) = &self.kind {
            *n
        } else {
            panic!("Not a number");
        }
    }
    pub fn new_int64(i: i64) -> Self {
        Self {
            hir_id: HirId(0),
            kind: LiteralKind::Number(i),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiteralKind {
    Number(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Array(Array),
    Char(char),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Array {
    pub values: Vec<Expression>,
}

impl Array {
    pub fn get_terminal_hir_id(&self) -> HirId {
        self.values.get(0).unwrap().get_hir_id()
    }
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
    Igt,
    Ige,
    Ilt,
    Ile,
    FEq,
    Fle,
    Fgt,
    Fge,
    Flt,
    BEq,
    Len,
}

impl std::fmt::Display for NativeOperatorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
