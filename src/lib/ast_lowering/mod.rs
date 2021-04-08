mod ast_lowering_context;
mod hir_map;

use crate::{ast::Root, hir, Config};
use ast_lowering_context::AstLoweringContext;
pub use hir_map::*;

pub fn lower_crate(config: &Config, root: &Root) -> hir::Root {
    let root = AstLoweringContext::new().lower_root(root);

    if config.show_hir {
        println!("{:#?}", root);
    }

    root
}
