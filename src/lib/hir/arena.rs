use std::collections::HashMap;

use paste::paste;

use super::{HasHirId, HirId, HirNode};
use crate::hir::visit::*;
use crate::{hir::visit::Visitor, hir::*};
use crate::{parser::Span, NodeId};

pub type Arena<'ar> = HashMap<HirId, HirNode<'ar>>;

#[derive(Debug, Default)]
pub struct HirNodeCollector<'ar> {
    arena: Arena<'ar>,
}

impl<'ar> HirNodeCollector<'ar> {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn take_arena(self) -> Arena<'ar> {
        self.arena
    }

    pub fn insert<T>(&mut self, node: &'ar T)
    where
        T: HasHirId,
        HirNode<'ar>: From<&'ar T>,
    {
        self.arena.insert(node.get_hir_id(), node.into());
    }
}

macro_rules! generate_hirnode_collector {
    ($($expr:ty,)+) => {
        impl<'ar> Visitor<'ar> for HirNodeCollector<'ar> {
            paste! {
                $(
                    fn [<visit_ $expr:snake>](&mut self, node: &'ar $expr) {
                        self.insert(node);

                        [<walk_ $expr:snake>](self, node);
                    }
                )+
            }
        }
    };
}

generate_hirnode_collector!(
    // Mod,
    // Root,
    TopLevel,
    // Trait,
    // Impl,
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
