use std::fmt;

use crate::infer::*;

use crate::ast::PrimitiveType;
// use crate::ast::Prototype;

#[derive(Debug, Clone, Hash, Eq, Serialize, Deserialize)]
pub enum Type {
    Primitive(PrimitiveType),
    // Proto(Box<Prototype>),
    FuncType(FuncType),
    Class(String),
    Trait(String),
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
            Self::Trait(t) => t.clone(),
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

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeSignature {
    pub args: Vec<Type>,
    pub ret: Type,
}

impl TypeSignature {
    pub fn apply_types(&self, orig: &Vec<Type>, dest: &Vec<Type>) -> Self {
        let applied_args = self
            .args
            .iter()
            .map(
                |arg_t| match orig.iter().enumerate().find(|(_, orig_t)| *orig_t == arg_t) {
                    Some((i, _orig_t)) => dest[i].clone(),
                    None => arg_t.clone(),
                },
            )
            .collect();

        let applied_ret = match orig
            .iter()
            .enumerate()
            .find(|(_, orig_t)| **orig_t == self.ret)
        {
            Some((i, _orig_t)) => dest[i].clone(),
            None => self.ret.clone(),
        };

        Self {
            args: applied_args,
            ret: applied_ret,
        }
    }
}
