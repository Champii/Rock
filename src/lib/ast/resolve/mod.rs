mod resolution_map;
mod resolve_ctx;

use std::collections::HashMap;

pub use resolution_map::*;
pub use resolve_ctx::*;

use crate::{ast::visit::*, parser::ParsingCtx, scopes::Scopes};

use super::{IdentifierPath, Root};

pub fn resolve(root: &mut Root, parsing_ctx: &mut ParsingCtx) {
    let mut scopes = HashMap::new();

    scopes.insert(IdentifierPath::new_root(), Scopes::new());

    let mut ctx = ResolveCtx {
        parsing_ctx,
        // scopes: Scopes::new(),
        scopes,
        cur_scope: IdentifierPath::new_root(),
        resolutions: ResolutionMap::default(),
    };

    ctx.visit_root(root);

    root.resolutions = ctx.resolutions;
}
