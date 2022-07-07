use std::collections::HashMap;

use crate::{
    ast::NodeId,
    helpers::*,
    infer::trait_solver::TraitSolver,
    parser::span::Span,
    resolver::ResolutionMap,
    ty::{FuncType, Type},
};

use super::{ast_print::AstPrintContext, visit::Visitor};

#[derive(Debug, Clone)]
pub struct Root {
    pub r#mod: Mod,
    pub resolutions: ResolutionMap<NodeId>,
    pub trait_solver: TraitSolver,
    pub operators_list: HashMap<String, u8>,
    pub unused: Vec<NodeId>,
    pub spans: HashMap<NodeId, Span>,
}

impl Root {
    pub fn new(r#mod: Mod) -> Self {
        Self {
            r#mod,
            resolutions: ResolutionMap::default(),
            operators_list: HashMap::new(),
            unused: vec![],
            spans: HashMap::new(),
            trait_solver: TraitSolver::new(),
        }
    }

    pub fn print(&self) {
        AstPrintContext::new().visit_root(self);
    }
}

#[derive(Debug, Clone)]
pub struct Mod {
    pub top_levels: Vec<TopLevel>,
}

impl Mod {
    pub fn new(top_levels: Vec<TopLevel>) -> Self {
        Self { top_levels }
    }
}

#[derive(Debug, Clone)]
pub enum TopLevel {
    Prototype(Prototype),
    Function(FunctionDecl),
    Trait(Trait),
    Impl(Impl),
    Struct(StructDecl),
    Mod(Identifier, Mod),
    Use(Use),
    Infix(Operator, u8),
}

impl TopLevel {
    pub fn new_function(f: FunctionDecl) -> Self {
        Self::Function(f)
    }

    pub fn new_infix(op: Operator, pred: u8) -> Self {
        Self::Infix(op, pred)
    }

    pub fn new_prototype(proto: Prototype) -> Self {
        Self::Prototype(proto)
    }

    pub fn new_use(u: Use) -> Self {
        Self::Use(u)
    }

    pub fn new_struct(s: StructDecl) -> Self {
        Self::Struct(s)
    }

    pub fn new_trait(t: Trait) -> Self {
        Self::Trait(t)
    }

    pub fn new_impl(t: Impl) -> Self {
        Self::Impl(t)
    }

    pub fn new_mod(ident: Identifier, mod_: Mod) -> Self {
        Self::Mod(ident, mod_)
    }
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: Identifier,
    pub defs: Vec<Prototype>,
}

impl StructDecl {
    pub fn new(name: Identifier, defs: Vec<Prototype>) -> Self {
        Self { name, defs }
    }
}

#[derive(Debug, Clone)]
pub struct StructCtor {
    pub name: Identifier,
    pub defs: HashMap<Identifier, Expression>,
}

impl StructCtor {
    pub fn new(name: Identifier, defs: HashMap<Identifier, Expression>) -> Self {
        Self { name, defs }
    }
}

#[derive(Debug, Clone)]
pub struct Trait {
    pub name: Type,
    pub types: Vec<Type>,
    pub defs: Vec<Prototype>,
    pub default_impl: Vec<FunctionDecl>,
}

impl Trait {
    pub fn new(
        name: Type,
        types: Vec<Type>,
        defs: Vec<Prototype>,
        default_impl: Vec<FunctionDecl>,
    ) -> Self {
        Self {
            name,
            types,
            defs,
            default_impl,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Impl {
    pub name: Type,
    pub types: Vec<Type>,
    pub defs: Vec<FunctionDecl>,
}

impl Impl {
    pub fn new(name: Type, types: Vec<Type>, defs: Vec<FunctionDecl>) -> Self {
        Self { name, types, defs }
    }
}

#[derive(Debug, Clone)]
pub struct Prototype {
    pub name: Identifier,
    pub signature: FuncType,
    pub node_id: NodeId,
}

impl Prototype {
    pub fn mangle(&mut self, prefix: String) {
        self.name.name = prefix + "_" + &self.name.name;
    }
}

#[derive(Debug, Clone)]
pub struct Use {
    pub path: IdentifierPath,
    pub node_id: NodeId,
}

impl Use {
    pub fn new(path: IdentifierPath, node_id: NodeId) -> Self {
        Self { path, node_id }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: Identifier,
    pub arguments: Vec<Identifier>,
    pub body: Body,
    pub node_id: NodeId,
    pub signature: FuncType,
}

impl FunctionDecl {
    pub fn new_self(
        node_id: NodeId,
        self_node_id: NodeId,
        name: Identifier,
        body: Body,
        mut arguments: Vec<Identifier>,
    ) -> Self {
        arguments.insert(0, Identifier::new("self".to_string(), self_node_id));
        Self {
            name,
            signature: FuncType::from_args_nb(arguments.len()),
            arguments,
            body,
            node_id,
        }
    }

    pub fn mangle(&mut self, prefixes: &[String]) {
        if prefixes.is_empty() {
            return;
        }

        self.name.name = prefixes.join("_") + "_" + &self.name.name;
    }
}

generate_has_name!(FunctionDecl);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct IdentifierPath {
    pub path: Vec<Identifier>,
}

impl IdentifierPath {
    pub fn new(path: Vec<Identifier>) -> Self {
        Self { path }
    }

    pub fn new_root() -> Self {
        Self {
            path: vec![Identifier {
                name: "root".to_string(),
                node_id: 0, // FIXME: should have a valid node_id ?
            }],
        }
    }

    pub fn has_root(&self) -> bool {
        self.path.len() > 0 && self.path[0].name == "root"
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

    pub fn last_segment_ref(&self) -> &Identifier {
        self.path.iter().last().unwrap()
    }

    pub fn prepend_mod(&self, path: IdentifierPath) -> Self {
        let mut path = path;

        path.path.extend::<_>(self.path.clone());

        path
    }

    pub fn resolve_supers(&mut self) {
        let to_remove = self
            .path
            .iter()
            .enumerate()
            .filter_map(
                |(i, name)| {
                    if name.name == *"super" {
                        Some(i)
                    } else {
                        None
                    }
                },
            )
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
    pub node_id: NodeId,
}

impl Identifier {
    pub fn new(name: String, node_id: NodeId) -> Self {
        Self { name, node_id }
    }
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

#[derive(Debug, Clone)]
pub struct Body {
    pub stmts: Vec<Statement>,
}

impl Body {
    pub fn new(stmts: Vec<Statement>) -> Self {
        Self { stmts }
    }

    pub fn with_return_self(&mut self, self_node_id: NodeId) {
        let identifier = Identifier::new("self".to_string(), self_node_id);
        let operand = Operand::new_identifier(identifier);
        // FIXME: should have a valid node_id ?
        let primary = PrimaryExpr::new(0, operand, vec![]);
        let unary = UnaryExpr::PrimaryExpr(primary);
        let expr = Expression::new_unary(unary);
        let stmt = Statement::new_expression(expr);

        self.stmts.push(stmt);
    }
}

#[derive(Debug, Clone)]
pub enum Statement {
    Expression(Box<Expression>),
    Assign(Box<Assign>),
    If(Box<If>),
    For(For),
}

impl Statement {
    pub fn new_expression(expr: Expression) -> Self {
        Self::Expression(Box::new(expr))
    }

    pub fn new_if(if_: If) -> Self {
        Self::If(Box::new(if_))
    }

    pub fn new_for(for_: For) -> Self {
        Self::For(for_)
    }

    pub fn new_assign(assign: Assign) -> Self {
        Self::Assign(Box::new(assign))
    }
}

#[derive(Debug, Clone)]
pub enum For {
    In(ForIn),
    While(While),
}

#[derive(Debug, Clone)]
pub struct While {
    pub predicat: Expression,
    pub body: Body,
}

impl While {
    pub fn new(predicat: Expression, body: Body) -> Self {
        Self { predicat, body }
    }
}

#[derive(Debug, Clone)]
pub struct ForIn {
    pub value: Identifier,
    pub expr: Expression,
    pub body: Body,
}

impl ForIn {
    pub fn new(value: Identifier, expr: Expression, body: Body) -> Self {
        Self { value, expr, body }
    }
}

#[derive(Debug, Clone)]
pub enum AssignLeftSide {
    Identifier(Expression),
    Indice(Expression),
    Dot(Expression),
}

#[derive(Debug, Clone)]
pub struct Assign {
    pub name: AssignLeftSide,
    pub value: Expression,
    pub is_let: bool,
}

impl Assign {
    pub fn new(name: AssignLeftSide, value: Expression, is_let: bool) -> Self {
        Self {
            name,
            value,
            is_let,
        }
    }
}

#[derive(Debug, Clone)]
pub struct If {
    pub node_id: NodeId,
    pub predicat: Expression,
    pub body: Body,
    pub else_: Option<Box<Else>>,
}

impl If {
    pub fn new(
        node_id: NodeId,
        predicat: Expression,
        body: Body,
        else_: Option<Box<Else>>,
    ) -> Self {
        Self {
            node_id,
            predicat,
            body,
            else_,
        }
    }

    pub fn get_flat(&self) -> Vec<(NodeId, Expression, Body)> {
        let mut res = vec![];

        res.push((self.node_id, self.predicat.clone(), self.body.clone()));

        if let Some(else_) = &self.else_ {
            res.extend(else_.get_flat());
        }

        res
    }

    pub fn last_else(&self) -> Option<&Body> {
        if let Some(else_) = &self.else_ {
            else_.last_else()
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub enum Else {
    If(If),
    Body(Body),
}

impl Else {
    pub fn get_flat(&self) -> Vec<(NodeId, Expression, Body)> {
        match self {
            Else::If(if_) => if_.get_flat(),
            Else::Body(_body) => vec![],
        }
    }

    pub fn last_else(&self) -> Option<&Body> {
        match self {
            Else::If(if_) => if_.last_else(),
            Else::Body(body) => Some(body),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    BinopExpr(UnaryExpr, Operator, Box<Expression>),
    UnaryExpr(UnaryExpr),
    NativeOperation(NativeOperator, Identifier, Identifier),
    StructCtor(StructCtor),
    Return(Box<Expression>), // NOTE: Shouldn't that be a statement?
}

impl Expression {
    #[allow(dead_code)]
    pub fn is_literal(&self) -> bool {
        match &self {
            Expression::UnaryExpr(unary) => unary.is_literal(),
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_identifier(&self) -> bool {
        match &self {
            Expression::UnaryExpr(unary) => unary.is_identifier(),
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_binop(&self) -> bool {
        matches!(&self, Expression::BinopExpr(_, _, _))
    }

    #[allow(dead_code)]
    pub fn is_indice(&self) -> bool {
        match &self {
            Expression::UnaryExpr(unary) => unary.is_indice(),
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_dot(&self) -> bool {
        match &self {
            Expression::UnaryExpr(unary) => unary.is_dot(),
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_return(&self) -> bool {
        matches!(&self, Expression::Return(_))
    }

    #[allow(dead_code)]
    pub fn new_unary(unary: UnaryExpr) -> Expression {
        Expression::UnaryExpr(unary)
    }

    #[allow(dead_code)]
    pub fn new_return(expr: Expression) -> Expression {
        Expression::Return(Box::new(expr))
    }

    #[allow(dead_code)]
    pub fn new_binop(unary: UnaryExpr, operator: Operator, expr: Expression) -> Expression {
        Expression::BinopExpr(unary, operator, Box::new(expr))
    }

    pub fn new_struct_ctor(ctor: StructCtor) -> Expression {
        Expression::StructCtor(ctor)
    }

    pub fn new_native_operator(
        operator: NativeOperator,
        id1: Identifier,
        id2: Identifier,
    ) -> Expression {
        Expression::NativeOperation(operator, id1, id2)
    }

    pub fn as_identifier(&self) -> Option<&Identifier> {
        match self {
            Expression::UnaryExpr(unary) => unary.as_identifier(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum UnaryExpr {
    PrimaryExpr(PrimaryExpr),
    #[allow(dead_code)]
    UnaryExpr(Operator, Box<UnaryExpr>), // Parser for that is not implemented yet
}

impl UnaryExpr {
    pub fn is_literal(&self) -> bool {
        match self {
            UnaryExpr::PrimaryExpr(p) => matches!(&p.op, Operand::Literal(_)),
            _ => false,
        }
    }

    pub fn is_identifier(&self) -> bool {
        match self {
            UnaryExpr::PrimaryExpr(p) => matches!(&p.op, Operand::Identifier(_)),
            _ => false,
        }
    }

    pub fn is_indice(&self) -> bool {
        match self {
            UnaryExpr::UnaryExpr(_, unary) => unary.is_indice(),
            UnaryExpr::PrimaryExpr(prim) => prim.is_indice(),
        }
    }

    pub fn is_dot(&self) -> bool {
        match self {
            UnaryExpr::UnaryExpr(_, unary) => unary.is_dot(),
            UnaryExpr::PrimaryExpr(prim) => prim.is_dot(),
        }
    }

    pub fn create_2_args_func_call(op: Operand, arg1: UnaryExpr, arg2: UnaryExpr) -> UnaryExpr {
        UnaryExpr::PrimaryExpr(PrimaryExpr {
            node_id: u64::MAX,
            op,
            secondaries: Some(vec![SecondaryExpr::Arguments(vec![
                Argument { arg: arg1 },
                Argument { arg: arg2 },
            ])]),
        })
    }

    pub fn new_primary(primary: PrimaryExpr) -> UnaryExpr {
        UnaryExpr::PrimaryExpr(primary)
    }

    pub fn as_identifier(&self) -> Option<&Identifier> {
        match self {
            UnaryExpr::PrimaryExpr(primary) => primary.as_identifier(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Operator(pub Identifier);

#[derive(Debug, Clone)]
pub struct PrimaryExpr {
    pub node_id: NodeId,
    pub op: Operand,
    pub secondaries: Option<Vec<SecondaryExpr>>,
}

impl PrimaryExpr {
    pub fn new(node_id: NodeId, op: Operand, secondaries: Vec<SecondaryExpr>) -> PrimaryExpr {
        PrimaryExpr {
            op,
            node_id,
            secondaries: if secondaries.len() == 0 {
                None
            } else {
                Some(secondaries)
            },
        }
    }

    #[allow(dead_code)]
    pub fn has_secondaries(&self) -> bool {
        self.secondaries.is_some()
    }

    pub fn is_indice(&self) -> bool {
        if let Some(secondaries) = &self.secondaries {
            secondaries.iter().any(|secondary| secondary.is_indice())
        } else {
            false
        }
    }

    pub fn is_dot(&self) -> bool {
        if let Some(secondaries) = &self.secondaries {
            secondaries.iter().any(|secondary| secondary.is_dot())
        } else {
            false
        }
    }

    pub fn as_identifier(&self) -> Option<&Identifier> {
        match &self.op {
            Operand::Identifier(id) => Some(id.path.last().unwrap()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Operand {
    Literal(Literal),
    Identifier(IdentifierPath),
    Expression(Box<Expression>), // parenthesis
}

impl Operand {
    pub fn new_identifier_path(id: IdentifierPath) -> Self {
        Self::Identifier(id)
    }

    pub fn new_identifier(id: Identifier) -> Self {
        Self::Identifier(IdentifierPath {
            path: vec![id.clone()],
        })
    }

    pub fn new_expression(expr: Expression) -> Self {
        Self::Expression(Box::new(expr))
    }

    #[allow(dead_code)]
    pub fn is_literal(&self) -> bool {
        matches!(&self, Operand::Literal(_))
    }

    #[allow(dead_code)]
    pub fn is_identifier(&self) -> bool {
        matches!(&self, Operand::Identifier(_))
    }

    pub fn new_literal(lit: Literal) -> Operand {
        Operand::Literal(lit)
    }

    pub fn from_identifier(id: Identifier) -> Operand {
        Operand::new_identifier(id)
    }

    pub fn desugar_self(&self, self_node_id: NodeId) -> (Operand, Option<SecondaryExpr>) {
        match self {
            Operand::Identifier(id) => {
                if id.path[0].name.len() > 1 && id.path[0].name.chars().nth(0).unwrap() == '@' {
                    let self_identifier = Identifier {
                        name: "self".to_string(),
                        node_id: self_node_id,
                    };

                    let new_op = Operand::new_identifier(self_identifier);

                    let mut new_id = id.path[0].clone();
                    new_id.name = new_id.name.replace("@", "");

                    let secondary = SecondaryExpr::Dot(new_id);

                    (new_op, Some(secondary))
                } else {
                    (self.clone(), None)
                }
            }
            _ => panic!("Cannot desugar self"),
        }
    }
}

impl Operand {
    #[allow(dead_code)]
    pub fn to_identifier_path(&self) -> IdentifierPath {
        if let Operand::Identifier(id) = self {
            id.clone()
        } else {
            panic!("Not an identifier path")
        }
    }
}

#[derive(Debug, Clone)]
pub enum SecondaryExpr {
    Arguments(Vec<Argument>),
    Indice(Box<Expression>), // Boxing here to keep the enum size low
    Dot(Identifier),
}

impl SecondaryExpr {
    pub fn is_indice(&self) -> bool {
        matches!(self, SecondaryExpr::Indice(_))
    }

    pub fn is_dot(&self) -> bool {
        matches!(self, SecondaryExpr::Dot(_))
    }

    // might be useful
    #[allow(dead_code)]
    pub fn is_arguments(&self) -> bool {
        matches!(self, SecondaryExpr::Arguments(_))
    }
}

#[derive(Debug, Clone)]
pub struct Literal {
    pub kind: LiteralKind,
    pub node_id: NodeId,
}

impl Literal {
    pub fn new_bool(b: bool, node_id: NodeId) -> Self {
        Self {
            kind: LiteralKind::Bool(b),
            node_id,
        }
    }

    pub fn new_number(num: i64, node_id: NodeId) -> Self {
        Self {
            kind: LiteralKind::Number(num),
            node_id,
        }
    }

    pub fn new_float(num: f64, node_id: NodeId) -> Self {
        Self {
            kind: LiteralKind::Float(num),
            node_id,
        }
    }

    pub fn new_array(arr: Array, node_id: NodeId) -> Self {
        Self {
            kind: LiteralKind::Array(arr),
            node_id,
        }
    }

    pub fn new_string(str: String, node_id: NodeId) -> Self {
        Self {
            kind: LiteralKind::String(str),
            node_id,
        }
    }

    pub fn new_char(c: char, node_id: NodeId) -> Self {
        Self {
            kind: LiteralKind::Char(c),
            node_id,
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self.kind {
            LiteralKind::Number(n) => n,
            _ => panic!("Not a Number"),
        }
    }

    pub fn as_str(&self) -> &str {
        match self.kind {
            LiteralKind::String(ref s) => s,
            _ => panic!("Not a String"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LiteralKind {
    Bool(bool),
    Number(i64),
    Float(f64),
    Array(Array),
    String(String),
    Char(char),
}

#[derive(Debug, Clone)]
pub struct Array {
    pub values: Vec<Expression>,
}

impl Array {
    pub fn new(values: Vec<Expression>) -> Self {
        Self { values }
    }
}

pub type Arguments = Vec<Argument>;

#[derive(Debug, Clone)]
pub struct Argument {
    pub arg: UnaryExpr,
}

impl Argument {
    pub fn new(arg: UnaryExpr) -> Self {
        Self { arg }
    }
}

#[derive(Debug, Clone)]
pub struct NativeOperator {
    pub kind: NativeOperatorKind,
    pub node_id: NodeId,
}

impl NativeOperator {
    pub fn new(node_id: NodeId, kind: NativeOperatorKind) -> Self {
        Self { kind, node_id }
    }
}

#[derive(Debug, Clone, PartialEq)]
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
    Fgt,
    Fge,
    Flt,
    Fle,
    BEq,
    Len,
}

impl NativeOperatorKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "IAdd" => Self::IAdd,
            "ISub" => Self::ISub,
            "IMul" => Self::IMul,
            "IDiv" => Self::IDiv,
            "FAdd" => Self::FAdd,
            "FSub" => Self::FSub,
            "FMul" => Self::FMul,
            "FDiv" => Self::FDiv,
            "IEq" => Self::IEq,
            "Igt" => Self::Igt,
            "Ige" => Self::Ige,
            "Ilt" => Self::Ilt,
            "Ile" => Self::Ile,
            "FEq" => Self::FEq,
            "Fgt" => Self::Fgt,
            "Fge" => Self::Fge,
            "Flt" => Self::Flt,
            "Fle" => Self::Fle,
            "BEq" => Self::BEq,
            "Len" => Self::Len,
            _ => panic!("Unknown native operator"),
        }
    }
}
