use std::collections::HashMap;

use crate::{
    ast::tree::{IdentifierPath, Root},
    diagnostics::Diagnostic,
    helpers::scopes::Scopes,
    infer::trait_solver::TraitSolver,
    parser::ParsingCtx,
};

mod resolution_map;
mod resolve_ctx;
mod unused_collector;

pub use resolution_map::*;
pub use resolve_ctx::*;

pub fn resolve(root: &mut Root, parsing_ctx: &mut ParsingCtx) -> Result<(), Diagnostic> {
    let mut scopes = HashMap::new();

    scopes.insert(IdentifierPath::new_root(), Scopes::new());

    let (resolutions, trait_solver) = {
        let mut ctx = ResolveCtx {
            parsing_ctx,
            scopes,
            cur_scope: IdentifierPath::new_root(),
            resolutions: ResolutionMap::default(),
            trait_solver: TraitSolver::new(),
        };

        ctx.run(root);

        (ctx.resolutions, ctx.trait_solver)
    };

    root.resolutions = resolutions;
    root.trait_solver = trait_solver;

    let (mut unused_fns, unused_methods) = unused_collector::collect_unused(root);

    for unused_fn in &unused_fns {
        let span = parsing_ctx.identities.get(unused_fn).unwrap();

        parsing_ctx
            .diagnostics
            .push_warning(Diagnostic::new_unused_function(span.clone()));
    }

    unused_fns.extend(unused_methods);

    parsing_ctx.return_if_error()
}
