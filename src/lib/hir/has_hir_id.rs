use crate::hir::*;
use paste::paste;

pub trait HasHirId {
    fn get_hir_id(&self) -> HirId;
}

macro_rules! impl_direct_get_hir_id_trait {
    ($(
        $name:ident
    )*) => {
        $(
            impl HasHirId for $name {
                fn get_hir_id(&self) -> HirId {
                    self.hir_id.clone()
                }
            }
        )*
    };
}

impl_direct_get_hir_id_trait!(
    Prototype
    FunctionDecl
    StructDecl
    Identifier
    If
    FunctionCall
    StructCtor
    Indice
    Dot
    Literal
    NativeOperator
);

macro_rules! impl_indirect_get_hir_id_trait {
    ($(
        $name:ident
    )*) => {
        paste! {
            $(
                impl HasHirId for $name {
                    fn get_hir_id(&self) -> HirId {
                        self.get_terminal_hir_id()
                    }
                }
            )*
        }
    };
}

impl_indirect_get_hir_id_trait!(
    TopLevel
    Statement
    Assign
    AssignLeftSide
    ArgumentDecl
    IdentifierPath
    FnBody
    For
    ForIn
    While
    Body
    Expression
    Array
    Else
);

impl<T: HasHirId> HasHirId for Vec<T> {
    fn get_hir_id(&self) -> HirId {
        self.last().unwrap().get_hir_id()
    }
}
