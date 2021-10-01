use std::collections::BTreeMap;

use paste::paste;

use crate::hir::visit::*;
use crate::hir::*;

#[derive(Debug, Default)]
pub struct Arena(BTreeMap<HirId, HirNode>);

impl Arena {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
    // pub fn get_all_of_type(&self, t: HirNode) -> Vec<&HirNode> {
    //     for (hir_id, hir_node) = self.0 {
    //         if let
    //     }
    // }
}
impl std::ops::Deref for Arena {
    type Target = BTreeMap<HirId, HirNode>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Arena {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Default)]
pub struct HirNodeCollector {
    arena: Arena,
}

impl HirNodeCollector {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn take_arena(self) -> Arena {
        self.arena
    }

    pub fn insert<T>(&mut self, node: &T)
    where
        T: HasHirId,
        T: Clone,
        hir_node::HirNode: From<T>, // HirNode: From<&T>,
    {
        self.arena.insert(node.get_hir_id(), node.clone().into());
    }
}

macro_rules! generate_hirnode_collector {
    ($($expr:ty,)+) => {
        impl<'a> Visitor<'a> for HirNodeCollector {
            paste! {
                $(
                    fn [<visit_ $expr:snake>](&mut self, node: &'a $expr) {
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

pub fn collect_arena(root: &Root) -> Arena {
    let mut hir_node_collector = HirNodeCollector::new();

    hir_node_collector.visit_root(root);

    hir_node_collector.take_arena()
}
