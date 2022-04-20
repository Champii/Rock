use std::collections::BTreeMap;

use paste::paste;

use crate::hir::visit::*;
use crate::hir::*;

//   This is not really an Arena, but more a HirNode Collector
// The name is confusing on its properties and usage,
// as one would expect the arena to own every hir nodes from their creation,
// and give (mut) references when needed.
//   Instead, this "Arena" is constructed from a clone of every nodes in the HIR,
// (as you can see bellow in the HirNodeCollector) and serves only as a global
// accessor to the graph's structure and immutable properties.
// Every effective mutable work is done on the hir::Root instead.
#[derive(Debug, Default)]
pub struct Arena(BTreeMap<HirId, HirNode>);

impl Arena {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
}

// FIXME: Code smell
impl std::ops::Deref for Arena {
    type Target = BTreeMap<HirId, HirNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// FIXME: Code smell
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
        hir_node::HirNode: From<T>,
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
