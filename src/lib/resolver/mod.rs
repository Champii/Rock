use crate::{
    ast::{span_collector::SpanCollector, visit::*, IdentifierPath, Root},
    diagnostics::Diagnostic,
    helpers::scopes::Scopes,
    parser::ParsingCtx,
    resolver::unused_collector::UnusedCollector,
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

    // find unused functions here and mark them as such to let them pass infer (to avoid crash)
    let mut unused_ctx = UnusedCollector::new(root.resolutions.clone());

    unused_ctx.visit_root(root);

    let (mut unused_fns, unused_methods) = unused_ctx.take_unused();

    let mut span_collector = SpanCollector::new();

    span_collector.visit_root(root);

    root.spans = span_collector.take_list();

    for unused_fn in &unused_fns {
        let span = root.spans.get(unused_fn).unwrap();

        parsing_ctx
            .diagnostics
            .push_warning(Diagnostic::new_unused_function(span.clone()));
    }

    unused_fns.extend(unused_methods);

    parsing_ctx.return_if_error()
}
