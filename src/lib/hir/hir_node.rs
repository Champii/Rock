use crate::{ast::Type, hir::HasHirId, hir::*};
use crate::{ast::TypeSignature, walk_list};

#[derive(Clone, Debug)]
pub enum HirNode {
    Root(Root),
    TopLevel(TopLevel),
    Trait(Trait),
    Impl(Impl),
    Assign(Assign),
    Prototype(Prototype),
    FunctionDecl(FunctionDecl),
    ArgumentDecl(ArgumentDecl),
    IdentifierPath(IdentifierPath),
    Identifier(Identifier),
    FnBody(FnBody),
    Body(Body),
    Statement(Statement),
    Expression(Expression),
    If(If),
    Else(Else),
    FunctionCall(FunctionCall),
    Literal(Literal),
    NativeOperator(NativeOperator),
}

impl HasHirId for HirNode {
    fn get_hir_id(&self) -> HirId {
        match self {
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
