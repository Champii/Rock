mod ast_lowering_context;
mod hir_map;
mod infix_desugar;
mod return_placement;

use crate::{
    ast::Root,
    hir::{self, HirNodeCollector},
};
use ast_lowering_context::AstLoweringContext;
pub use hir_map::*;
pub use infix_desugar::*;

pub fn lower_crate(root: &Root) -> hir::Root {
    let mut root =
        AstLoweringContext::new(root.operators_list.clone(), root.unused.clone()).lower_root(root);

    let mut hir_node_collector = HirNodeCollector::new();

    use crate::hir::visit::Visitor;
    hir_node_collector.visit_root(&root);

    let arena = hir_node_collector.take_arena();

    println!("ARENA {:#?}", arena);

    root
}
