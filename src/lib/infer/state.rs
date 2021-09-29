use std::collections::{BTreeMap, HashMap};

use crate::{
    ast::{FuncType, Type},
    diagnostics::{Diagnostic, Diagnostics},
    hir::*,
    parser::Span,
};

pub type NodeId = u64;

pub type TypeId = u64;

pub type Env = BTreeMap<HirId, Type>;

#[derive(Debug, Default, Clone)]
pub struct Envs {
    fns: BTreeMap<HirId, HashMap<FuncType, Env>>,
    current_fn: (HirId, FuncType),
    pub spans: HashMap<HirId, Span>,
    pub diagnostics: Diagnostics,
}

impl Envs {
    pub fn new(diagnostics: Diagnostics, spans: HashMap<HirId, Span>) -> Self {
        Self {
            diagnostics,
            spans,
            ..Self::default()
        }
    }
    pub fn get_current_env_mut(&mut self) -> Option<&mut BTreeMap<HirId, Type>> {
        self.fns
            .get_mut(&self.current_fn.0)?
            .get_mut(&self.current_fn.1)
    }

    pub fn get_current_env(&self) -> Option<&BTreeMap<HirId, Type>> {
        self.fns
            .get(&self.current_fn.0)
            .and_then(|map| map.get(&self.current_fn.1))
    }

    pub fn set_current_fn(&mut self, f: (HirId, FuncType)) -> bool {
        // if !f.1.are_args_solved() {
        //     self.diagnostics.push_error(Diagnostic::new_unresolved_type(
        //         self.spans.get(&f.0).unwrap().clone(),
        //         f.1.to_func_type(),
        //     ));

        //     return false;
        // }

        self.fns
            .entry(f.0.clone())
            .or_insert_with(|| HashMap::new())
            .entry(f.1.clone())
            .or_insert_with(|| Env::default());

        self.current_fn = f;

        return true;
    }

    pub fn get_current_fn(&self) -> (HirId, FuncType) {
        self.current_fn.clone()
    }

    pub fn set_type(&mut self, dest: &HirId, src: &Type) {
        if let Type::ForAll(_) = src {
            warn!("set_type requires `src: &Type` to be solved");

            return;
        }

        let previous = self
            .get_current_env_mut()
            .unwrap()
            .insert(dest.clone(), src.clone());

        match (src, previous.clone()) {
            (Type::FuncType(src_f), Some(Type::FuncType(prev_f))) if !src_f.eq(&prev_f) => {
                if prev_f.is_solved() && src_f.is_solved() {
                    self.diagnostics.push_error(Diagnostic::new_type_conflict(
                        self.spans.get(dest).unwrap().clone(),
                        src.clone(),
                        previous.clone().unwrap(),
                        src.clone(),
                        previous.clone().unwrap(),
                    ));
                }
            }
            (src, Some(previous)) if !src.eq(&previous) => {
                self.diagnostics.push_error(Diagnostic::new_type_conflict(
                    self.spans.get(dest).unwrap().clone(),
                    src.clone(),
                    previous.clone(),
                    src.clone(),
                    previous,
                ));
            }
            _ => (),
        }
    }

    pub fn set_type_eq(&mut self, dest: &HirId, src: &HirId) {
        self.set_type(dest, &self.get_type(src).unwrap().clone())
    }

    pub fn get_type(&self, hir_id: &HirId) -> Option<&Type> {
        self.get_current_env().and_then(|env| env.get(hir_id))
    }

    pub fn apply_args_type(&mut self, f: &FunctionDecl) {
        let eq_types = f
            .arguments
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                (
                    arg.get_hir_id(),
                    self.current_fn.1.arguments.get(i).unwrap().clone(),
                )
            })
            .collect::<Vec<_>>();

        eq_types.into_iter().for_each(|(id, t)| {
            self.set_type(&id, &t);
        });
    }

    pub fn get_fn_types(&self, f: &HirId) -> Option<&HashMap<FuncType, Env>> {
        self.fns.get(f)
    }

    pub fn get_inner(&self) -> &BTreeMap<HirId, HashMap<FuncType, Env>> {
        &self.fns
    }

    pub fn add_empty(&mut self, hir_id: &HirId) {
        self.fns
            .entry(hir_id.clone())
            .or_insert_with(|| HashMap::new());
    }

    pub fn amend_current_sig(&mut self, new_sig: &FuncType) {
        if self.current_fn.1 == *new_sig {
            return;
        }

        let env = self.get_current_env().unwrap().clone();

        self.fns
            .get_mut(&self.current_fn.0)
            .unwrap()
            .insert(new_sig.clone(), env);

        self.fns
            .get_mut(&self.current_fn.0)
            .unwrap()
            .remove(&self.current_fn.1);

        self.current_fn.1 = new_sig.clone();
    }

    pub fn get_diagnostics(self) -> Diagnostics {
        self.diagnostics
    }
}
