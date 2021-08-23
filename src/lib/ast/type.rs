use std::fmt;

use crate::infer::*;

use crate::ast::PrimitiveType;
// use crate::ast::Prototype;

#[derive(Debug, Clone)]
pub enum Type {
    Primitive(PrimitiveType),
    // Proto(Box<Prototype>),
    FuncType(FuncType),
    Class(String),
    ForAll(String), // TODO
    Undefined(u64),
}

impl PartialEq for Type {
    fn eq(&self, other: &Type) -> bool {
        self.get_name() == other.get_name()
    }
}

impl Type {
    pub fn get_name(&self) -> String {
        match self {
            Self::Primitive(p) => p.get_name(),
            // Self::Proto(p) => p.name.clone().unwrap_or(String::new()),
            Self::FuncType(f) => f.name.clone(),
            Self::Class(c) => c.clone(),
            Self::ForAll(_) => String::new(),
            Self::Undefined(s) => s.to_string(),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}

#[derive(Debug, Clone)]
pub struct FuncType {
    pub name: String,
    pub arguments: Vec<TypeId>,
    pub ret: TypeId,
}

impl FuncType {
    pub fn new(name: String, arguments: Vec<TypeId>, ret: TypeId) -> Self {
        Self {
            name,
            arguments,
            ret,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeSignature {
    pub args: Vec<Type>,
    pub ret: Type,
}
