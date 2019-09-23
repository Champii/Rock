use std::collections::HashMap;

use super::context::*;

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub top_levels: Vec<TopLevel>,
}

#[derive(Debug, Clone)]
pub enum TopLevel {
    Mod(String),
    Class(Class),
    Function(FunctionDecl),
    Prototype(Prototype),
}

#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub class_attributes: Vec<Attribute>, // [(name, type, default)]
    pub methods: Vec<FunctionDecl>,
    pub class_methods: Vec<FunctionDecl>,
}

impl Class {
    pub fn get_attribute(&self, name: String) -> Option<(Attribute, usize)> {
        let mut i: usize = 0;

        for attr in self.attributes.clone() {
            if name == attr.name {
                return Some((attr.clone(), i));
            }

            i += 1;
        }

        None
    }

    pub fn get_method(&self, name: String) -> Option<FunctionDecl> {
        for method in self.methods.clone() {
            if name == method.name {
                return Some(method.clone());
            }
        }

        None
    }
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub t: Option<Type>,
    pub default: Option<Expression>,
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub ret: Option<Type>,
    pub arguments: Vec<ArgumentDecl>,
    pub body: Body,
    pub class_name: Option<String>,
}

impl FunctionDecl {
    pub fn add_this_arg(&mut self) {
        let names: Vec<&str> = self.name.split("_").collect();

        self.arguments.insert(
            0,
            ArgumentDecl {
                name: "this".to_string(),
                t: Some(Type::Name(names[0].to_string())),
            },
        )
    }

    pub fn is_solved(&self) -> bool {
        self.arguments.iter().all(|arg| arg.t.is_some()) && self.ret.is_some()
    }

    pub fn apply_name(&mut self, t: Vec<TypeInfer>, ret: Option<Type>) {
        let mut name = String::new();

        for ty in t {
            name = name + &ty.get_ret().unwrap().get_name();
        }

        if let Some(t) = self.ret.clone() {
            name = name + &t.get_name();
        }

        self.name = self.name.clone() + &name;
    }

    pub fn apply_types(&mut self, ret: Option<Type>, t: Vec<TypeInfer>) {
        self.apply_name(t.clone(), ret.clone());

        self.ret = ret;

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
    pub ret: Type,
    pub arguments: Vec<Type>,
}

impl Prototype {
    pub fn apply_name(&mut self) {
        let mut name = String::new();

        for ty in &self.arguments {
            name = name + &ty.get_name();
        }

        self.name = Some(self.name.clone().unwrap() + &name);
    }
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
    For(For),
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
pub enum For {
    In(ForIn),
    While(While),
}

#[derive(Debug, Clone)]
pub struct ForIn {
    pub value: String,
    pub expr: Expression,
    pub body: Body,
}

#[derive(Debug, Clone)]
pub struct While {
    pub predicat: Expression,
    pub body: Body,
}

#[derive(Debug, Clone)]
pub struct Assignation {
    pub name: String,
    pub t: Option<Type>,
    pub value: Box<Statement>,
}

#[derive(Debug, Clone)]
pub enum Expression {
    BinopExpr(UnaryExpr, Operator, Box<Expression>),
    UnaryExpr(UnaryExpr),
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
    Selector((String, u8, Option<Type>)), // . Identifier  // u8 is the attribute index in struct // option<Type> is the class type if needed
    Arguments(Vec<Argument>),             // (Expr, Expr, ...)
    Index(Box<Expression>),               // [Expr]
}

#[derive(Debug, Clone)]
pub enum Operand {
    Literal(Literal),
    Identifier(String),
    ClassInstance(ClassInstance),
    // PrimaryExpr(Box<PrimaryExpr>, SecondaryExpr),
    Array(Array),
    Expression(Box<Expression>), // parenthesis
}

#[derive(Debug, Clone)]
pub struct ClassInstance {
    pub name: String,
    pub class: Class,
    pub attributes: HashMap<String, Attribute>,
}

impl ClassInstance {
    pub fn get_attribute(&self, name: String) -> Option<(Attribute, usize)> {
        let mut i: usize = 0;

        for (_, attr) in self.attributes.clone() {
            if name == attr.name {
                return Some((attr.clone(), i));
            }

            i += 1;
        }

        None
    }
}

#[derive(Debug, Clone)]
pub enum Literal {
    Number(u64),
    String(String),
    Bool(u64),
}

#[derive(Debug, Clone)]
pub struct Array {
    pub items: Vec<Expression>,
    pub t: Option<Type>,
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

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Name(String),
    Array(Box<Type>, usize),
}

impl Type {
    pub fn get_name(&self) -> String {
        match self {
            Type::Name(s) => s.clone(),
            Type::Array(a, n) => "[]".to_string() + &a.get_name(),
        }
    }

    pub fn get_inner(&self) -> Type {
        match self {
            r @ Type::Name(_) => r.clone(),
            Type::Array(a, n) => a.get_inner(),
        }
    }
}
