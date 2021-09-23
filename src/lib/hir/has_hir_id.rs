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
    Identifier
    If
    FunctionCall
    Indice
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
    Assign
    ArgumentDecl
    IdentifierPath
    FnBody
    Body
    Statement
    Expression
    Array
    Else
);
