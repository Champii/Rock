mod resolution_map;
mod resolve_ctx;
mod unused_collector;

use std::collections::HashMap;

pub use resolution_map::*;
pub use resolve_ctx::*;

use crate::{
    ast::{
        ast_print::AstPrintContext, resolve::unused_collector::UnusedCollector, visit::*, TopLevel,
        TopLevelKind,
    },
    diagnostics::Diagnostic,
    helpers::scopes::Scopes,
    parser::ParsingCtx,
};

use super::{IdentifierPath, Root};

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

    // TODO: find unused functions here and mark them as such to let them pass infer (to avoid crash)
    let mut unused_ctx = UnusedCollector::new(root.resolutions.clone());

    unused_ctx.visit_root(root);

    let unused = unused_ctx.take_unused();

    println!("unused {:?}", unused);

    // let filter_unused_top_levels = |top_level| {
    //     match &top_level.kind {
    //         TopLevelKind::Function(f) => {
    //             if unused.contains(&f.identity.node_id) {
    //                 error!("Unused function {:?}", f.name);

    //                 return None;
    //             }
    //         }
    //         TopLevelKind::Trait(t) => {
    //             let mut defs = vec![];

    //             for f in &t.defs {
    //                 if unused.contains(&f.identity.node_id) {
    //                     unused_trait_method_names.push(f.name.clone());

    //                     error!("Unused trait method {:?}", f.name);
    //                 } else {
    //                     defs.push(f.clone());
    //                 }
    //             }

    //             if defs.is_empty() {
    //                 return None;
    //             }

    //             let mut t2 = t.clone();
    //             t2.defs = defs.clone();

    //             return Some(TopLevel {
    //                 kind: TopLevelKind::Trait(t2),
    //                 ..top_level.clone()
    //             });
    //         }
    //         TopLevelKind::Impl(i) => {
    //             let mut defs = vec![];

    //             for f in &i.defs {
    //                 if unused_trait_method_names.contains(&f.name) {
    //                     error!("Unused impl method {:?}", f.name);
    //                 } else {
    //                     defs.push(f.clone());
    //                 }
    //             }

    //             if defs.is_empty() {
    //                 return None;
    //             }

    //             let mut i2 = i.clone();
    //             i2.defs = defs.clone();

    //             return Some(TopLevel {
    //                 kind: TopLevelKind::Impl(i2),
    //                 ..top_level.clone()
    //             });
    //         }
    //         _ => (),
    //     };

    //     Some(top_level.clone())
    // };

    root.r#mod.filter_unused_top_levels(unused);

    parsing_ctx.print_ast(root);

    parsing_ctx.return_if_error()
}
