use crate::Error;
use crate::Parser;

#[macro_use]
pub mod helper;
#[macro_use]
pub mod ast_print;
mod argument;
mod argument_decl;
mod identity;
// mod array;
// mod assignation;
// mod attribute;
mod body;
// mod class;
// mod class_instance;
mod r#else;
mod expression;
// mod r#for;
// mod forin;
mod function_decl;
mod identifier;
mod r#if;
mod literal;
mod operand;
mod operator;
mod primary_expr;
mod primitive_type;
// mod prototype;
mod secondary_expr;
// mod selector;
mod source_file;
mod statement;
mod string;
mod top_level;
mod r#type;
mod unary_expr;
// mod r#while;

pub use argument::{Argument, Arguments};
pub use argument_decl::{ArgumentDecl, ArgumentsDecl};
pub use identity::Identity;
// pub use array::Array;
// pub use assignation::Assignation;
// pub use attribute::Attribute;
pub use body::Body;
// pub use class::Class;
// pub use class_instance::ClassInstance;
pub use expression::{Expression, ExpressionKind};
// pub use forin::ForIn;
pub use function_decl::FunctionDecl;
pub use identifier::Identifier;
pub use literal::{Literal, LiteralKind};
pub use operand::{Operand, OperandKind};
pub use operator::Operator;
pub use primary_expr::PrimaryExpr;
pub use primitive_type::PrimitiveType;
// pub use prototype::Prototype;
pub use r#else::Else;
// pub use r#for::For;
pub use r#if::If;
pub use r#type::Type;
// pub use r#while::While;
pub use secondary_expr::SecondaryExpr;
// pub use selector::Selector;
pub use source_file::SourceFile;
pub use statement::{Statement, StatementKind};
pub use top_level::{TopLevel, TopLevelKind};
pub use unary_expr::UnaryExpr;

pub trait Parse {
    fn parse(ctx: &mut Parser) -> Result<Self, Error>
    where
        Self: Sized;
}
