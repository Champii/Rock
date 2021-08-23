use crate::ast::Type;

#[derive(Debug, Clone)]
pub enum PrimitiveType {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    Float64,
    String(usize),
    Array(Box<Type>, usize),
}

impl PrimitiveType {
    pub fn get_name(&self) -> String {
        match self {
            Self::Void => "Void".to_string(),
            Self::Bool => "Bool".to_string(),
            Self::Int8 => "Int8".to_string(),
            Self::Int16 => "Int16".to_string(),
            Self::Int32 => "Int32".to_string(),
            Self::Int64 => "Int64".to_string(),
            Self::Float64 => "Float64".to_string(),
            Self::String(size) => format!("String({})", size),
            Self::Array(t, size) => format!("[{}; {}]", t.get_name(), size),
        }
    }

    pub fn from_name(s: &str) -> Option<PrimitiveType> {
        match s {
            "Void" => Some(Self::Void),
            "Bool" => Some(Self::Bool),
            "Int8" => Some(Self::Int8),
            "Int16" => Some(Self::Int16),
            "Int32" => Some(Self::Int32),
            "Int64" => Some(Self::Int64),
            "Float64" => Some(Self::Float64),
            "String" => Some(Self::String(0)),
            _ => None,
        }
    }
}
