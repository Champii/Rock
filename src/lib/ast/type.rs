use std::fmt;

use colored::*;

use crate::{ast::PrimitiveType, hir};

use super::{FuncType, StructType};

#[derive(Clone, Eq, Serialize, Deserialize)]
pub enum Type {
    Primitive(PrimitiveType),
    FuncType(FuncType),
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
            Type::FuncType(ft) => ft.is_solved(),
            Type::Struct(_) => true,
            Type::Trait(_) => true,
            Type::ForAll(_) => false,
            Type::Undefined(_) => false,
        }
    }

    pub fn is_primitive(&self) -> bool {
        matches!(self, Self::Primitive(_x))
    }

    pub fn is_bool(&self) -> bool {
        self.is_primitive()
            .then(|| self.into_primitive().is_bool())
            .unwrap_or(false)
    }

    pub fn is_int8(&self) -> bool {
        self.is_primitive()
            .then(|| self.into_primitive().is_int8())
            .unwrap_or(false)
    }

    pub fn is_int16(&self) -> bool {
        self.is_primitive()
            .then(|| self.into_primitive().is_int16())
            .unwrap_or(false)
    }

    pub fn is_int32(&self) -> bool {
        self.is_primitive()
            .then(|| self.into_primitive().is_int32())
            .unwrap_or(false)
    }

    pub fn is_int64(&self) -> bool {
        self.is_primitive()
            .then(|| self.into_primitive().is_int64())
            .unwrap_or(false)
    }

    pub fn is_float64(&self) -> bool {
        self.is_primitive()
            .then(|| self.into_primitive().is_float64())
            .unwrap_or(false)
    }

    pub fn is_string(&self) -> bool {
        self.is_primitive()
            .then(|| self.into_primitive().is_string())
            .unwrap_or(false)
    }

    pub fn is_array(&self) -> bool {
        self.is_primitive()
            .then(|| self.into_primitive().is_array())
            .unwrap_or(false)
    }

    pub fn is_struct(&self) -> bool {
        matches!(self, Self::Struct(_x))
    }

    pub fn is_trait(&self) -> bool {
        matches!(self, Self::Trait(_x))
    }

    pub fn is_func(&self) -> bool {
        matches!(self, Self::FuncType(_x))
    }

    pub fn is_forall(&self) -> bool {
        matches!(self, Self::ForAll(_x))
    }

    pub fn get_name(&self) -> String {
        match self {
            Self::Primitive(p) => p.get_name(),
            Self::FuncType(f) => format!("{:?}", f),
            Self::Struct(s) => s.name.clone(),
            Self::Trait(t) => t.clone(),
            Self::ForAll(n) => String::from(n),
            Self::Undefined(s) => s.to_string(),
        }
    }

    pub fn into_struct_type(&self) -> StructType {
        if let Type::Struct(t) = self {
            t.clone()
        } else {
            panic!("Not a struct type");
        }
    }

    pub fn into_func_type(&self) -> FuncType {
        if let Type::FuncType(f) = self {
            f.clone()
        } else {
            panic!("Not a func type");
        }
    }

    pub fn into_primitive(&self) -> PrimitiveType {
        if let Type::Primitive(p) = self {
            p.clone()
        } else {
            panic!("Not a primitive");
        }
    }
}

impl fmt::Debug for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::FuncType(f) => format!("{:?}", f),
            Self::Struct(s) => format!("{:?}", s),
            _ => self.get_name().cyan().to_string(),
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
        Type::FuncType(t)
    }
}

impl From<&FuncType> for Type {
    fn from(t: &FuncType) -> Self {
        Type::FuncType(t.clone())
    }
}

impl From<StructType> for Type {
    fn from(t: StructType) -> Self {
        Type::Struct(t)
    }
}

impl From<super::StructDecl> for Type {
    fn from(t: super::StructDecl) -> Self {
        StructType::from(t).into()
    }
}

impl From<&super::StructDecl> for Type {
    fn from(t: &super::StructDecl) -> Self {
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
        if t.len() == 1 && (t.chars().nth(0).unwrap()).is_lowercase() {
            Type::ForAll(t)
        } else {
            Type::Trait(t)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_type_signature() {
        let sig = FuncType::from_args_nb(2);

        assert_eq!(*sig.arguments[0], Type::forall("a"));
        assert_eq!(*sig.arguments[1], Type::forall("b"));
        assert_eq!(*sig.ret, Type::forall("c"));
    }

    #[test]
    fn apply_forall_types() {
        let sig = FuncType::from_args_nb(2);

        let res = sig.apply_forall_types(&vec![Type::forall("b")], &vec![Type::int64()]);

        assert_eq!(*res.arguments[0], Type::forall("a"));
        assert_eq!(*res.arguments[1], Type::int64());
        assert_eq!(*res.ret, Type::forall("c"));
    }

    #[test]
    fn apply_types() {
        let sig = FuncType::from_args_nb(2);

        let res = sig.apply_types(vec![Type::int64()], Type::int64());

        assert_eq!(*res.arguments[0], Type::int64());
        assert_eq!(*res.arguments[1], Type::forall("b"));
        assert_eq!(*res.ret, Type::int64());
    }

    #[test]
    fn apply_partial_types() {
        let sig = FuncType::from_args_nb(2);

        let res = sig.apply_partial_types(&vec![None, Some(Type::int64())], Some(Type::int64()));

        assert_eq!(*res.arguments[0], Type::forall("a"));
        assert_eq!(*res.arguments[1], Type::int64());
        assert_eq!(*res.ret, Type::int64());
    }
}
