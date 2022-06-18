use super::Type;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum PrimitiveType {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    Int, // not a real type, default to Int64
    Float64,
    String,
    Array(Box<Type>, usize),
    Char,
}

impl PrimitiveType {
    pub fn is_solved(&self) -> bool {
        if let PrimitiveType::Array(t, _) = self {
            t.is_solved()
        } else {
            true
        }
    }

    pub fn get_name(&self) -> String {
        match self {
            Self::Void => "Void".to_string(),
            Self::Bool => "Bool".to_string(),
            Self::Int8 => "Int8".to_string(),
            Self::Int16 => "Int16".to_string(),
            Self::Int32 => "Int32".to_string(),
            Self::Int64 => "Int64".to_string(),
            Self::Int => "Int64".to_string(),
            Self::Float64 => "Float64".to_string(),
            Self::String => "String".to_string(),
            Self::Array(t, _size) => format!("[{}]", t.get_name()),
            Self::Char => "Char".to_string(),
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
            "Int" => Some(Self::Int64),
            "Float64" => Some(Self::Float64),
            "String" => Some(Self::String),
            "Char" => Some(Self::Char),
            _ => None,
        }
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, PrimitiveType::Bool)
    }

    pub fn is_int8(&self) -> bool {
        matches!(self, PrimitiveType::Int8)
    }

    pub fn is_int16(&self) -> bool {
        matches!(self, PrimitiveType::Int16)
    }

    pub fn is_int32(&self) -> bool {
        matches!(self, PrimitiveType::Int32)
    }

    pub fn is_int64(&self) -> bool {
        matches!(self, PrimitiveType::Int64)
    }

    pub fn is_int(&self) -> bool {
        matches!(self, PrimitiveType::Int)
    }

    pub fn is_concrete_int(&self) -> bool {
        self.is_int8() || self.is_int16() || self.is_int32() || self.is_int64()
    }

    pub fn is_float64(&self) -> bool {
        matches!(self, PrimitiveType::Float64)
    }

    pub fn is_string(&self) -> bool {
        matches!(self, PrimitiveType::String)
    }

    pub fn is_array(&self) -> bool {
        matches!(self, PrimitiveType::Array(_, _))
    }

    pub fn is_char(&self) -> bool {
        matches!(self, PrimitiveType::Char)
    }

    pub fn try_as_array(&self) -> Option<(Type, usize)> {
        if let PrimitiveType::Array(t, s) = self {
            Some((*t.clone(), *s))
        } else {
            None
        }
    }
}
