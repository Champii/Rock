mod annotate;
mod call_collector;
mod call_solver;
mod constraint;
mod mangle;
mod monomorphizer;
mod proto_collector;
mod state;

use crate::{
    diagnostics::Diagnostic,
    hir::{hir_printer::*, visit_mut::*, Arena},
    infer::mangle::*,
    parser::ParsingCtx,
    Config,
};

use self::annotate::AnnotateContext;
use self::constraint::ConstraintContext;
pub use self::state::*;

pub fn infer(
    root: &mut crate::hir::Root,
    parsing_ctx: &mut ParsingCtx,
    config: &Config,
) -> Result<(), Diagnostic> {
    let infer_state = InferState::new(root);

    let mut annotate_ctx = AnnotateContext::new(infer_state, root.trait_methods.clone());

    annotate_ctx.annotate(&root);

    let mut constraint_ctx = ConstraintContext::new(annotate_ctx.get_state(), &root);

    constraint_ctx.constraint(&root);

    let (mut infer_state, mut new_resolutions) = constraint_ctx.get_state();

    // Here: get protos, calls, and make a map of proto application from these calls

    if let Err(diags) = infer_state.solve() {
        for diag in diags {
            parsing_ctx.diagnostics.push_error(diag.clone());
        }
    }

    // // FIXME : We need to do two pass to get a complete resolve in some case
    // let infer_state = if !infer_state.is_solved() {
    //     //
    //     let mut constraint_ctx = ConstraintContext::new(infer_state, &root);

    //     constraint_ctx.constraint(&root);

    //     let (mut infer_state, new_resolutions2) = constraint_ctx.get_state();

    //     new_resolutions.extend(new_resolutions2);

    //     // // // FIXME: don't
    //     // for (k, v) in new_resolutions {
    //     //     root.resolutions.insert(k.clone(), v.clone());
    //     // }

    //     if let Err(diags) = infer_state.solve() {
    //         for diag in diags {
    //             parsing_ctx.diagnostics.push_error(diag.clone());
    //         }
    //     }

    //     infer_state
    // } else {
    //     infer_state
    // };

    let trait_call_to_mangle = infer_state.trait_call_to_mangle.clone();

    let node_types = infer_state.get_node_types();

    let mut protos = proto_collector::collect_prototypes(root, &mut infer_state);

    println!("PROTOS {:#?}", protos);

    let calls = call_collector::collect_calls(root);
    println!("CALLS {:#?}", calls);

    // TODO: Monomorphize before inference ?
    let bindings = call_solver::solve_calls(protos, calls, root, &mut infer_state);
    println!("SOLVED {:#?}", bindings);

    if config.show_state {
        println!("ROOT {:#?}", root);
        println!("STATE {:#?}", infer_state);
    }

    // here we consume infer_state
    let (types, diags) = infer_state.get_types();

    let mut new_root = monomorphizer::monomophize(bindings, root);

    new_root.hir_map = root.hir_map.clone();
    new_root.spans = root.spans.clone();

    // Infer again
    let infer_state = InferState::new(&new_root);

    let mut annotate_ctx = AnnotateContext::new(infer_state, new_root.trait_methods.clone());

    annotate_ctx.annotate(&new_root);

    let mut constraint_ctx = ConstraintContext::new(annotate_ctx.get_state(), &new_root);

    constraint_ctx.constraint(&new_root);

    let (mut infer_state, mut new_resolutions) = constraint_ctx.get_state();

    // Here: get protos, calls, and make a map of proto application from these calls

    if let Err(diags) = infer_state.solve() {
        for diag in diags {
            parsing_ctx.diagnostics.push_error(diag.clone());
        }
    }

    let node_types = infer_state.get_node_types();
    println!("NEW INFER STATE {:#?}", infer_state);
    let (types, diags) = infer_state.get_types();
    new_root.node_types = node_types;
    // new_root.trait_call_to_mangle = trait_call_to_mangle;
    new_root.types = types;
    println!("NEW HIR {:#?}", new_root);
    super::hir::hir_printer::print(&new_root);

    //

    // let mut mangle_ctx = MangleContext {
    //     trait_call_to_mangle: trait_call_to_mangle.clone(),
    // };

    // root.node_types = node_types;
    // root.trait_call_to_mangle = trait_call_to_mangle;
    // mangle_ctx.visit_root(root);

    // // we dont exit on unresolved types .. yet
    // for diag in diags {
    //     parsing_ctx.diagnostics.push_warning(diag);
    // }

    // // // FIXME: don't
    // for (k, v) in new_resolutions {
    //     root.resolutions.insert(k.clone(), v.clone());
    // }

    // root.types = types;

    if config.show_hir {
        use crate::hir::visit::*;

        let mut printer = HirPrinter::new(root);

        printer.visit_root(&root);
    }

    parsing_ctx.return_if_error()?;

    Ok(())
}
