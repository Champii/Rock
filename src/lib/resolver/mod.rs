use crate::{
    ast::{span_collector, visit::*, IdentifierPath, Root},
    diagnostics::Diagnostic,
    helpers::scopes::Scopes,
    parser::ParsingCtx,
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

    root.spans = span_collector::collect_spans(root);

    for unused_fn in &unused_fns {
        let span = root.spans.get(unused_fn).unwrap();

        parsing_ctx
            .diagnostics
            .push_warning(Diagnostic::new_unused_function(span.clone()));
    }

    unused_fns.extend(unused_methods);

    parsing_ctx.return_if_error()
}
