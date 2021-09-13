mod ast_lowering_context;
mod hir_map;
mod infix_desugar;
mod return_placement;

use crate::{
    ast::Root,
    hir::{self, Arena, HirNodeCollector},
};
use ast_lowering_context::AstLoweringContext;
pub use hir_map::*;
pub use infix_desugar::*;

pub fn lower_crate(root: &Root) -> hir::Root {
    AstLoweringContext::new(root.operators_list.clone(), root.unused.clone()).lower_root(root)
}
