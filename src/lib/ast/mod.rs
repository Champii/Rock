use crate::Error;
use crate::Parser;
use crate::Token;

mod argument;
mod argument_decl;
mod array;
mod assignation;
mod body;
mod class;
mod class_instance;
mod r#else;
mod expression;
mod r#for;
mod forin;
mod function_decl;
mod identifier;
mod r#if;
mod literal;
mod operand;
mod operator;
mod primary_expr;
mod prototype;
mod secondary_expr;
mod selector;
mod source_file;
mod statement;
mod top_level;
mod r#type;
mod unary_expr;
mod r#while;

pub use argument::{Argument, Arguments};
pub use argument_decl::{ArgumentDecl, ArgumentsDecl};
pub use array::Array;
pub use assignation::Assignation;
pub use body::Body;
pub use class::Class;
pub use class_instance::ClassInstance;
pub use expression::{Expression, ExpressionKind};
pub use forin::ForIn;
pub use function_decl::FunctionDecl;
pub use identifier::Identifier;
pub use literal::Literal;
pub use operand::{Operand, OperandKind};
pub use operator::Operator;
pub use primary_expr::PrimaryExpr;
pub use prototype::Prototype;
pub use r#else::Else;
pub use r#for::For;
pub use r#if::If;
pub use r#type::{Type, TypeInfer};
pub use r#while::While;
pub use secondary_expr::SecondaryExpr;
pub use selector::Selector;
pub use source_file::SourceFile;
pub use statement::{Statement, StatementKind};
pub use top_level::TopLevel;
pub use unary_expr::UnaryExpr;

pub trait Parse {
    fn parse(ctx: &mut Parser) -> Result<Self, Error>
    where
        Self: Sized;
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub t: Option<Type>,
    pub default: Option<Expression>,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct ArgumentType {
    pub t: Type,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub enum PrimitiveType {
    Void,
    Bool,
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    String(usize),
    Array(Box<Type>, usize),
}

impl PrimitiveType {
    pub fn get_name(&self) -> String {
        match self {
            Self::Void => "Void".to_string(),
            Self::Bool => "Bool".to_string(),
            Self::Int => "Int".to_string(),
            Self::Int8 => "Int8".to_string(),
            Self::Int16 => "Int16".to_string(),
            Self::Int32 => "Int32".to_string(),
            Self::Int64 => "Int64".to_string(),
            Self::String(size) => format!("String({})", size),
            Self::Array(t, size) => format!("[{}; {}]", t.get_name(), size),
        }
    }

    pub fn from_name(s: &String) -> Option<PrimitiveType> {
        match s.as_ref() {
            "Void" => Some(Self::Void),
            "Bool" => Some(Self::Bool),
            "Int" => Some(Self::Int),
            "Int8" => Some(Self::Int8),
            "Int16" => Some(Self::Int16),
            "Int32" => Some(Self::Int32),
            "Int64" => Some(Self::Int64),
            _ => None,
        }
    }
}
