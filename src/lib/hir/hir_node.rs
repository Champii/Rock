use crate::{ast::Type, hir::HasHirId, hir::*};
use crate::{ast::TypeSignature, walk_list};

#[derive(Clone, Debug)]
pub enum HirNode<'ar> {
    Mod(&'ar Mod),
    TopLevel(&'ar TopLevel),
    Trait(&'ar Trait),
    Impl(&'ar Impl),
    Assign(&'ar Assign),
    Prototype(&'ar Prototype),
    FunctionDecl(&'ar FunctionDecl),
    ArgumentDecl(&'ar ArgumentDecl),
    IdentifierPath(&'ar IdentifierPath),
    Identifier(&'ar Identifier),
    FnBody(&'ar FnBody),
    Body(&'ar Body),
    Statement(&'ar Statement),
    Expression(&'ar Expression),
    If(&'ar If),
    Else(&'ar Else),
    FunctionCall(&'ar FunctionCall),
    Literal(&'ar Literal),
    NativeOperator(&'ar NativeOperator),
}

impl<'ar> HasHirId for HirNode<'ar> {
    fn get_hir_id(&self) -> HirId {
        match self {
            HirNode::Mod(x) => x.get_hir_id(),
            HirNode::Assign(x) => x.get_hir_id(),
            HirNode::Prototype(x) => x.get_hir_id(),
            HirNode::FunctionDecl(x) => x.get_hir_id(),
            HirNode::ArgumentDecl(x) => x.get_hir_id(),
            HirNode::IdentifierPath(x) => x.get_hir_id(),
            HirNode::Identifier(x) => x.get_hir_id(),
            HirNode::FnBody(x) => x.get_hir_id(),
            HirNode::Body(x) => x.get_hir_id(),
            HirNode::Statement(x) => x.get_hir_id(),
            HirNode::Expression(x) => x.get_hir_id(),
            HirNode::If(x) => x.get_hir_id(),
            HirNode::Else(x) => x.get_hir_id(),
            HirNode::FunctionCall(x) => x.get_hir_id(),
            HirNode::Literal(x) => x.get_hir_id(),
            HirNode::NativeOperator(x) => x.get_hir_id(),
            _ => unimplemented!(),
        }
    }
}

macro_rules! generate_hirnode_from {
    ($($expr:ident,)+) => {
        $(
            impl<'ar> From<&'ar $expr> for HirNode<'ar> {
                fn from(expr: &'ar $expr) -> HirNode<'ar> {
                    HirNode::$expr(expr)
                }
            }
        )+
    };
}

generate_hirnode_from!(
    Mod,
    TopLevel,
    Trait,
    Impl,
    Assign,
    Prototype,
    FunctionDecl,
    ArgumentDecl,
    IdentifierPath,
    Identifier,
    FnBody,
    Body,
    Statement,
    Expression,
    If,
    Else,
    FunctionCall,
    Literal,
    NativeOperator,
);
