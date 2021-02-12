mod ast_lowering_context;
mod hir_map;

use crate::{ast::Root, hir};
use ast_lowering_context::AstLoweringContext;
pub use hir_map::*;

pub fn lower_crate(root: &Root) -> hir::Root {
    AstLoweringContext::new().lower_root(root)
}
