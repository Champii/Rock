use std::collections::BTreeMap;

use crate::{ast::TypeSignature, hir::*};

use super::{call_collector::Calls, proto_collector::Protos};

#[derive(Debug, Clone)]
pub struct FnCall {
    pub call_site_id: HirId,
    pub call_hir_id: HirId,
    // pub sig: TypeSignature,
    pub fc: FunctionCall,
}

pub type Bindings = BTreeMap<
    HirId, //proto_id
    (
        TypeSignature, //proto_sig
        Vec<FnCall>,   // ((call_site_id, call_id), (call_sig, fc))
    ),
>;

pub fn bind_calls_to_proto(protos: &Protos, calls: &Calls, root: &Root) -> Bindings {
    let mut res = Bindings::new();

    // println!("Protos {:#?}", protos);
    // println!("Calls {:#?}", calls);
    for (proto_hir_id, proto_sig) in protos {
        res.insert(proto_hir_id.clone(), (proto_sig.clone(), Vec::new()));
    }

    for ((call_site_id, call_hir_id), fc) in calls.clone() {
        // println!(
        //     "Call site: {:?}, call_hir_id {:?}, fc {:#?} root reso {:#?}",
        //     call_site_id, call_site_id, fc, root.resolutions
        // );
        let (_proto_sig, map) = res
            .get_mut(&root.resolutions.get(&call_hir_id).unwrap())
            .unwrap();

        map.push(FnCall {
            call_site_id,
            call_hir_id,
            // sig: proto_sig.clone(),
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
        .map(|(_proto_hir_id, (proto_sig, calls))| {
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
    let empty_bindings: Bindings = bindings
        .iter()
        .map(|(id, (sig, _calls))| (id.clone(), (sig.clone(), Vec::new())))
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
        // FIXME: This prevent recursion
        if target_proto_id == proto_hir_id {
            continue;
        }

        new_bindings = process_proto(&bindings, target_proto_id.clone(), new_bindings, root);
    }

    new_bindings
}

pub fn solve_calls(protos: Protos, calls: Calls, root: &Root) -> Bindings {
    // Objective: get a list of fully resolved typesignatures for top_level function decl
    let bindings = bind_calls_to_proto(&protos, &calls, root);

    let bindings = monomorphize(bindings.clone(), find_main_proto(root), root);

    bindings
}
