use std::collections::HashMap;

use crate::ids::{IdGen, TypeVarId};
use crate::types::Type;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DeclarationBinding {
    pub(crate) ty: Type,
    pub(crate) mutable: bool,
}

#[derive(Clone, Default)]
pub(crate) struct DeclarationScope {
    bindings: HashMap<String, DeclarationBinding>,
}

impl DeclarationScope {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn define(&mut self, name: String, ty: Type, mutable: bool) {
        self.bindings
            .insert(name, DeclarationBinding { ty, mutable });
    }

    pub(crate) fn lookup(&self, name: &str) -> Option<&DeclarationBinding> {
        self.bindings.get(name)
    }
}

pub struct DeclarationTypeVars {
    next_var: IdGen<TypeVarId>,
}

impl DeclarationTypeVars {
    pub(crate) fn new() -> Self {
        Self {
            next_var: IdGen::new(),
        }
    }

    pub(crate) fn fresh_type_var(&mut self) -> Type {
        Type::TypeVar(self.next_var.fresh())
    }

    pub(crate) fn next_raw(&self) -> u32 {
        self.next_var.next_raw()
    }
}

impl Default for DeclarationTypeVars {
    fn default() -> Self {
        Self::new()
    }
}
