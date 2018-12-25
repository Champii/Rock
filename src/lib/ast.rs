use super::context::*;

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub top_levels: Vec<TopLevel>,
}

#[derive(Debug, Clone)]
pub enum TopLevel {
    Mod(String),
    Function(FunctionDecl),
    Prototype(Prototype),
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub t: Option<Type>,
    pub arguments: Vec<ArgumentDecl>,
    pub body: Body,
}

impl FunctionDecl {
    // pub fn get_args_types(&self) -> Vec<Option<Type>> {
    //     //
    // }

    pub fn apply_types(&mut self, ret: Option<Type>, t: Vec<TypeInfer>) {
        self.t = ret;

        let mut i = 0;

        for arg in &mut self.arguments {
            if i >= t.len() {
                break;
            }

            arg.t = t[i].get_ret();

            i += 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Prototype {
    pub name: Option<String>,
    pub t: Type,
    pub arguments: Vec<Type>,
}

#[derive(Debug, Clone)]
pub struct ArgumentType {
    pub t: Type,
}

#[derive(Debug, Clone)]
pub struct ArgumentDecl {
    pub name: String,
    pub t: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct Argument {
    pub arg: Expression,
}

#[derive(Debug, Clone)]
pub struct Body {
    pub stmts: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    If(If),
    Expression(Expression),
    Assignation(Assignation),
}

#[derive(Debug, Clone)]
pub struct If {
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
pub enum Expression {
    BinopExpr(UnaryExpr, Operator, Box<Expression>),
    UnaryExpr(UnaryExpr),
}

#[derive(Debug, Clone)]
pub struct Assignation {
    pub name: String,
    pub t: Option<Type>,
    pub value: Box<Statement>,
}

#[derive(Debug, Clone)]
pub enum UnaryExpr {
    PrimaryExpr(PrimaryExpr),
    UnaryExpr(Operator, Box<UnaryExpr>),
}

#[derive(Debug, Clone)]
pub enum PrimaryExpr {
    PrimaryExpr(Operand, Vec<SecondaryExpr>),
}

#[derive(Debug, Clone)]
pub enum SecondaryExpr {
    Selector(String),         // . Identifier
    Arguments(Vec<Argument>), // (Expr, Expr, ...)
    Index(Box<Expression>),   // [Expr]
}

#[derive(Debug, Clone)]
pub enum Operand {
    Literal(Literal),
    Identifier(String),
    // PrimaryExpr(Box<PrimaryExpr>, SecondaryExpr),
    Expression(Box<Expression>), // parenthesis
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub left: Box<Expression>,
    pub op: Operator,
    pub right: Box<Operation>,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Number(u64),
    String(String),
    Bool(u64),
}

#[derive(Debug, Clone)]
pub enum Operator {
    Add,
    Sub,
    Sum,
    Div,
    Mod,

    EqualEqual,
    DashEqual,
}

#[derive(Debug, Clone)]
pub enum Type {
    Name(String),
    Array(Box<Type>),
}
