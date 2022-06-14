use std::fmt;

use crate::{ast, hir};

use super::{FuncType, PrimitiveType, StructType};

#[derive(Clone, Eq, Serialize, Deserialize)]
pub enum Type {
    Primitive(PrimitiveType),
    Func(FuncType),
    Struct(StructType),
    Trait(String),
    ForAll(String),
    Undefined(u64), // FIXME: To remove
}

impl std::hash::Hash for Type {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Type) -> bool {
        self.get_name() == other.get_name()
    }
}

macro_rules! generate_primitive_is_check {
    ($e:tt) => {
        pub fn $e(&self) -> bool {
            self.try_as_primitive_type()
                .map(|p| p.$e())
                .unwrap_or(false)
        }
    };
}

macro_rules! generate_primitive_checks {
    ($($e:tt),+) => {
        $(
            generate_primitive_is_check!($e);
        )+
    };
}

impl Type {
    pub fn int64() -> Self {
        Self::Primitive(PrimitiveType::Int64)
    }

    pub fn forall(t: &str) -> Self {
        Self::ForAll(String::from(t))
    }

    pub fn is_solved(&self) -> bool {
        match self {
            Type::Primitive(p) => p.is_solved(),
            Type::Func(ft) => ft.is_solved(),
            Type::Struct(_) => true,
            Type::Trait(_) => true,
            Type::ForAll(_) => false,
            Type::Undefined(_) => false,
        }
    }

    pub fn is_primitive(&self) -> bool {
        matches!(self, Self::Primitive(_x))
    }

    generate_primitive_checks!(
        is_bool, is_int8, is_int16, is_int32, is_float64, is_string, is_array
    );

    pub fn is_struct(&self) -> bool {
        matches!(self, Self::Struct(_x))
    }

    pub fn is_trait(&self) -> bool {
        matches!(self, Self::Trait(_x))
    }

    pub fn is_func(&self) -> bool {
        matches!(self, Self::Func(_x))
    }

    pub fn is_forall(&self) -> bool {
        matches!(self, Self::ForAll(_x))
    }

    pub fn get_name(&self) -> String {
        match self {
            Self::Primitive(p) => p.get_name(),
            Self::Func(f) => format!("{:?}", f),
            Self::Struct(s) => s.name.clone(),
            Self::Trait(t) => t.clone(),
            Self::ForAll(n) => String::from(n),
            Self::Undefined(s) => s.to_string(),
        }
    }

    pub fn as_struct_type(&self) -> StructType {
        if let Type::Struct(t) = self {
            t.clone()
        } else {
            panic!("Not a struct type");
        }
    }

    pub fn as_func_type(&self) -> FuncType {
        if let Type::Func(f) = self {
            f.clone()
        } else {
            panic!("Not a func type");
        }
    }

    pub fn as_primitive_type(&self) -> PrimitiveType {
        if let Type::Primitive(p) = self {
            p.clone()
        } else {
            panic!("Not a primitive");
        }
    }

    pub fn try_as_struct_type(&self) -> Option<StructType> {
        match self {
            Type::Struct(t) => Some(t.clone()),
            _ => None,
        }
    }

    pub fn try_as_func_type(&self) -> Option<FuncType> {
        match self {
            Type::Func(f) => Some(f.clone()),
            _ => None,
        }
    }

    pub fn try_as_primitive_type(&self) -> Option<PrimitiveType> {
        match self {
            Type::Primitive(p) => Some(p.clone()),
            _ => None,
        }
    }
}

impl fmt::Debug for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Primitive(p) => format!("{:?}", p),
            Self::Func(f) => format!("{:?}", f),
            Self::Struct(s) => format!("{:?}", s),
            Self::Trait(t) => format!("Trait {:?}", t),
            Self::ForAll(t) => format!("forall. {:?}", t),
            Self::Undefined(t) => format!("UNDEFINED {:?}", t),
            // _ => self.get_name().cyan().to_string(),
        };

        write!(f, "{}", s)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}

impl From<PrimitiveType> for Type {
    fn from(t: PrimitiveType) -> Self {
        Type::Primitive(t)
    }
}

impl From<FuncType> for Type {
    fn from(t: FuncType) -> Self {
        Type::Func(t)
    }
}

impl From<&FuncType> for Type {
    fn from(t: &FuncType) -> Self {
        Type::Func(t.clone())
    }
}

impl From<StructType> for Type {
    fn from(t: StructType) -> Self {
        Type::Struct(t)
    }
}

impl From<ast::StructDecl> for Type {
    fn from(t: ast::StructDecl) -> Self {
        StructType::from(t).into()
    }
}

impl From<&ast::StructDecl> for Type {
    fn from(t: &ast::StructDecl) -> Self {
        StructType::from(t.clone()).into()
    }
}

impl From<hir::StructDecl> for Type {
    fn from(t: hir::StructDecl) -> Self {
        Type::Struct(t.into())
    }
}

impl From<&hir::StructDecl> for Type {
    fn from(t: &hir::StructDecl) -> Self {
        t.clone().into()
    }
}

impl From<String> for Type {
    fn from(t: String) -> Self {
        if t.len() == 1 && (t.chars().next().unwrap()).is_lowercase() {
            Type::ForAll(t)
        } else if t.chars().next().unwrap() == '[' {
            Type::Primitive(PrimitiveType::Array(
                Box::new(Type::from(
                    t.trim_matches('[').trim_matches(']').to_string(),
                )),
                0, // FIXME
            ))
        } else if PrimitiveType::from_name(&t).is_some() {
            Type::Primitive(PrimitiveType::from_name(&t).unwrap())
        } else {
            Type::Trait(t)
        }
    }
}
