use crate::{ast::tree::Root, hir};

mod ast_lowering_context;
mod hir_map;
mod infix_desugar;

use ast_lowering_context::AstLoweringContext;
pub use hir_map::*;
pub use infix_desugar::*;

pub fn lower_crate(root: &Root) -> hir::Root {
    AstLoweringContext::new(root.operators_list.clone()).lower_root(root)
}
