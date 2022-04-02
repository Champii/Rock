use crate::{
    ast::{tree2::IdentifierPath, tree2::Root, visit2::*},
    diagnostics::Diagnostic,
    helpers::scopes::Scopes,
    parser::{ParsingCtx, Span},
};

mod resolution_map;
mod resolve_ctx;
mod unused_collector;

use std::collections::HashMap;

pub use resolution_map::*;
pub use resolve_ctx::*;

pub fn resolve(root: &mut Root, parsing_ctx: &mut ParsingCtx) -> Result<(), Diagnostic> {
    let mut scopes = HashMap::new();

    scopes.insert(IdentifierPath::new_root(), Scopes::new());

    let mut ctx = ResolveCtx {
        parsing_ctx,
        scopes,
        cur_scope: IdentifierPath::new_root(),
        resolutions: ResolutionMap::default(),
    };

    ctx.visit_root(root);

    root.resolutions = ctx.resolutions;

    let (mut unused_fns, unused_methods) = unused_collector::collect_unused(root);

    for unused_fn in &unused_fns {
        let span = parsing_ctx.identities.get(unused_fn).unwrap();

        parsing_ctx
            .diagnostics
            .push_warning(Diagnostic::new_unused_function(Span::from(span.clone())));
    }

    unused_fns.extend(unused_methods);

    parsing_ctx.return_if_error()
}
