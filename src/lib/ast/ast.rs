use super::Identity;

pub struct Root {
    pub r#mod: Mod,
    pub identity: Identity,
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
    Function(FunctionDecl),
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

#[derive(Debug, Clone, Default)]
pub struct ArgumentDecl {
    pub name: String,
    pub identity: Identity,
}

#[derive(Debug, Clone)]
pub struct Body {
    pub stmt: Statement,
    pub identity: Identity,
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: Box<StatementKind>,
    pub identity: Identity,
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
    pub identity: Identity,
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

#[derive(Debug, Clone)]
pub struct Operand {
    pub kind: OperandKind,
    pub identity: Identity,
}

#[derive(Debug, Clone)]
pub enum OperandKind {
    Literal(Literal),
    Identifier(Identifier),
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
    pub identity: Identity,
}
