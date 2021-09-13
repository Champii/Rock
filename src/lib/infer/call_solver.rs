use std::collections::HashMap;

use crate::{
    ast::{Type, TypeSignature},
    hir::HirId,
    InferState,
};
use crate::{hir::visit::*, hir::*};

use super::{call_collector::Calls, proto_collector::Protos};

#[derive(Debug, Clone)]
pub struct FnCall {
    pub call_site_id: HirId,
    pub call_hir_id: HirId,
    pub sig: TypeSignature,
    pub fc: FunctionCall,
}

pub type Bindings = HashMap<
    HirId, //proto_id
    (
        TypeSignature, //proto_sig
        Vec<FnCall>,   // ((call_site_id, call_id), (call_sig, fc))
    ),
>;

pub fn bind_calls_to_proto(protos: &Protos, calls: &Calls, root: &Root) -> Bindings {
    let mut res = Bindings::new();

    for (proto_hir_id, proto_sig) in protos {
        res.insert(proto_hir_id.clone(), (proto_sig.clone(), Vec::new()));
    }

    for ((call_site_id, call_hir_id), fc) in calls.clone() {
        let (proto_sig, map) = res
            .get_mut(&root.resolutions.get_recur(&call_hir_id).unwrap())
            .unwrap();

        map.push(FnCall {
            call_site_id,
            call_hir_id,
            sig: proto_sig.clone(),
            fc,
        });
    }

    res
}

fn find_main_proto(root: &Root) -> HirId {
    root.get_function_by_name("main").unwrap().hir_id
}

fn get_calls_by_call_site(bindings: &Bindings, call_site: HirId) -> Vec<(FnCall, TypeSignature)> {
    bindings
        .iter()
        .map(|(proto_hir_id, (proto_sig, calls))| {
            calls
                .iter()
                .filter_map(|fn_call| {
                    if fn_call.call_site_id == call_site {
                        Some((fn_call.clone(), proto_sig.clone()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect::<Vec<_>>()
}

fn monomorphize(bindings: Bindings, entry_point_hir_id: HirId, root: &Root) -> Bindings {
    let mut empty_bindings: Bindings = bindings
        .iter()
        .map(|(id, (sig, calls))| (id.clone(), (sig.clone(), Vec::new())))
        .collect();

    process_proto(&bindings, entry_point_hir_id.clone(), empty_bindings, root)
}

fn process_proto(
    bindings: &Bindings,
    proto_hir_id: HirId,
    mut new_bindings: Bindings,
    root: &Root,
) -> Bindings {
    let entry_refs = get_calls_by_call_site(&bindings, proto_hir_id.clone());

    for (fn_call, proto_sig) in entry_refs {
        let target_proto_id = root.resolutions.get_recur(&fn_call.call_hir_id).unwrap();

        new_bindings
            .entry(target_proto_id.clone())
            .or_insert((proto_sig.clone(), Vec::new()))
            .1
            .push(fn_call.clone());

        // avoid loops
        if target_proto_id == proto_hir_id {
            continue;
        }

        new_bindings = process_proto(&bindings, target_proto_id.clone(), new_bindings, root);
    }

    new_bindings
}

pub fn solve_calls(
    protos: Protos,
    calls: Calls,
    root: &Root,
    infer_state: &mut InferState,
) -> Bindings {
    // Objective: get a list of fully resolved typesignatures for top_level function decl
    //
    let mut bindings = bind_calls_to_proto(&protos, &calls, root);

    println!("Bindings {:#?}", bindings);

    let mut bindings = monomorphize(bindings.clone(), find_main_proto(root), root);
    println!("MONOMORPH {:#?}", bindings);

    // let mut res = protos
    //     .iter()
    //     .filter(|(_, t_sig)| t_sig.is_solved())
    //     .map(|(hir_id, t_sig)| (hir_id.clone(), vec![t_sig.clone()]))
    //     .collect::<HashMap<_, _>>();

    let mut res = HashMap::new();

    return bindings;

    // // let mut to_process = protos
    // //     .into_iter()
    // //     .filter(|(_, t_sig)| !t_sig.is_solved())
    // //     .collect::<HashMap<_, _>>();

    // // let mut calls = calls.clone();
    // // let mut stack = calls.clone();

    // // let mut to_process_old = to_process.clone();

    // let mut i = 0;

    // // let mut bindings_copy = bindings.clone();

    // while !bindings.is_empty() && i < 3 {
    //     // let mut to_add = HashMap::new();

    //     bindings = bindings
    //         .into_iter()
    //         .filter_map(|(proto_hir_id, (proto_sig, calls))| {
    //             if proto_sig.is_solved() {
    //                 res.entry(proto_hir_id.clone())
    //                     .or_insert_with(Vec::new)
    //                     .push(proto_sig.clone());

    //                 return None;
    //             }

    //             let calls = calls
    //                 .into_iter()
    //                 .filter_map(|(call_hir_id, (call_sig, fc))| {
    //                     let args_t = fc
    //                         .args
    //                         .iter()
    //                         .map(|arg| {
    //                             infer_state
    //                                 .get_type(infer_state.get_type_id(arg.get_hir_id()).unwrap())
    //                         })
    //                         .collect::<Vec<_>>();

    //                     let ret_t =
    //                         infer_state.get_type(infer_state.get_type_id(fc.get_hir_id()).unwrap());

    //                     let mut t_sig = call_sig.clone();
    //                     t_sig.apply_partial_types_mut(&args_t, ret_t);

    //                     println!("POST PARTIAL APPLY {:#?}", t_sig);
    //                     if t_sig.is_solved() {
    //                         // infer and typecheck here
    //                         //
    //                         // assign fresh hir_id
    //                         // update resolutions
    //                         // add annotations
    //                         // add constraints
    //                         // solve
    //                         // update resolutions

    //                         res.entry(proto_hir_id.clone())
    //                             .or_insert_with(Vec::new)
    //                             .push(t_sig.clone());

    //                         None
    //                     } else {
    //                         Some((call_hir_id, (t_sig, fc)))
    //                     }
    //                 })
    //                 .collect::<Vec<_>>();

    //             // (calls.len() > 0).then(|| (proto_hir_id, (proto_sig, calls)))
    //             (true).then(|| (proto_hir_id, (proto_sig, calls)))
    //         })
    //         .collect();

    //     // for (proto_hir_id, t_sig) in to_add {
    //     // bindings.entry(proto_hir_id)
    //     // }

    //     i += 1;
    // }

    // loop {
    //     i = i + 1;

    //     if to_process.is_empty() {
    //         break;
    //     }

    //     if to_process == to_process_old && i > 3 {
    //         println!("BREAK {:#?}", res);
    //         // panic!("No progress in solve calls");
    //         break;
    //     }

    //     to_process_old = to_process.clone();

    //     calls = calls
    //         .into_iter()
    //         .filter(|(hir_id, fc)| {
    //             if let Some(reso_id) = root.resolutions.get(&hir_id) {
    //                 match to_process.get(&reso_id) {
    //                     Some(t_sig) => {
    //                         if t_sig.is_solved() {
    //                             res.entry(hir_id.clone())
    //                                 .or_insert_with(Vec::new)
    //                                 .push(t_sig.clone());

    //                             return false;
    //                         } else {
    //                             let args_t = fc
    //                                 .args
    //                                 .iter()
    //                                 .map(|arg| {
    //                                     infer_state.get_type(
    //                                         infer_state.get_type_id(arg.get_hir_id()).unwrap(),
    //                                     )
    //                                 })
    //                                 .collect::<Vec<_>>();

    //                             let ret_t = infer_state
    //                                 .get_type(infer_state.get_type_id(fc.get_hir_id()).unwrap());

    //                             let mut t_sig = t_sig.clone();
    //                             t_sig.apply_partial_types_mut(&args_t, ret_t);

    //                             println!("POST PARTIAL APPLY {:#?}", t_sig);
    //                             if t_sig.is_solved() {
    //                                 res.entry(hir_id.clone())
    //                                     .or_insert_with(Vec::new)
    //                                     .push(t_sig.clone());

    //                                 return false;
    //                             } else {
    //                                 to_process.insert(hir_id.clone(), t_sig);
    //                                 // stack.insert(hir_id, fc);
    //                             }

    //                             // zx
    //                         }
    //                         true
    //                     }
    //                     None => {
    //                         panic!("WOUAT");
    //                     }
    //                 }
    //             } else {
    //                 panic!("WOUAT2");
    //             }
    //         })
    //         .collect();

    //     // for (hir_id, fc) in &calls {
    //     //    if let Some(reso_id) = root.resolutions.get(&hir_id) {
    //     //        match to_process.get(&reso_id) {
    //     //            Some(t_sig) => {
    //     //                if t_sig.is_solved() {
    //     //                    res.entry(hir_id.clone())
    //     //                        .or_insert_with(Vec::new)
    //     //                        .push(t_sig.clone());
    //     //                } else {
    //     //                    let args_t = fc
    //     //                        .args
    //     //                        .iter()
    //     //                        .map(|arg| {
    //     //                            infer_state.get_type(
    //     //                                infer_state.get_type_id(arg.get_hir_id()).unwrap(),
    //     //                            )
    //     //                        })
    //     //                        .collect::<Vec<_>>();

    //     //                    let mut t_sig = t_sig.clone();
    //     //                    t_sig.apply_partial_types_mut(&args_t);

    //     //                    println!("POST PARTIAL APPLY {:#?}", t_sig);
    //     //                    if t_sig.is_solved() {
    //     //                        res.entry(hir_id.clone())
    //     //                            .or_insert_with(Vec::new)
    //     //                            .push(t_sig.clone());
    //     //                    } else {
    //     //                        to_process.insert(&hir_id, t_sig);
    //     //                        // stack.insert(hir_id, fc);
    //     //                    }

    //     //                    // zx
    //     //                }
    //     //            }
    //     //            None => {
    //     //                panic!("WOUAT");
    //     //                // root.get_trait_method(
    //     //                //     fc.op.as_identifier().name,
    //     //                //     infer_state
    //     //                //         .get_type(
    //     //                //             infer_state
    //     //                //                 .get_type_id(fc.args.first().unwrap().get_hir_id())
    //     //                //                 .unwrap(),
    //     //                //         )
    //     //                //         .unwrap(),
    //     //                // );
    //     //                let node = root.arena.get(&reso_id).unwrap();
    //     //                println!("RESO NODE {:#?}", node);
    //     //            } // is a trait
    //     //        }
    //     //    }
    //     // }
    // }

    res
}
