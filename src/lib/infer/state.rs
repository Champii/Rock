use std::collections::{BTreeMap, HashMap};

use crate::{
    diagnostics::{Diagnostic, Diagnostics},
    hir::*,
    parser::span2::Span,
    ty::*,
};

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
        self.fns
            .entry(f.0.clone())
            .or_insert_with(HashMap::new)
            .entry(f.1.clone())
            .or_insert_with(Env::default);

        self.current_fn = f;

        true
    }

    pub fn get_current_fn(&self) -> (HirId, FuncType) {
        self.current_fn.clone()
    }

    fn set_type_alone(&mut self, dest: &HirId, src: &Type) -> Option<Type> {
        if let Type::ForAll(_) = src {
            warn!("set_type requires `src: &Type` to be solved");

            return None;
        }

        self.get_current_env_mut()
            .unwrap()
            .insert(dest.clone(), src.clone())
    }

    pub fn set_type(&mut self, dest: &HirId, src: &Type) {
        let previous = self.set_type_alone(dest, src);

        match (src, previous.clone()) {
            (Type::Func(src_f), Some(Type::Func(prev_f))) if !src_f.eq(&prev_f) => {
                if prev_f.is_solved() && src_f.is_solved() {
                    self.diagnostics.push_error(Diagnostic::new_type_conflict(
                        self.spans.get(dest).unwrap().clone().into(),
                        previous.clone().unwrap(),
                        src.clone(),
                        previous.unwrap(),
                        src.clone(),
                    ));
                }
            }
            // generic Int subtyping
            (Type::Primitive(src_prim), Some(Type::Primitive(PrimitiveType::Int)))
                if src_prim.is_concrete_int() =>
            {
                error!(
                    "Replacing hir_id {:?} type {:?} with {:?}",
                    dest, previous, src
                );
            }
            (src, Some(previous)) if !src.eq(&previous) => {
                if previous.is_solved() && src.is_solved() {
                    self.diagnostics.push_error(Diagnostic::new_type_conflict(
                        self.spans.get(dest).unwrap().clone().into(),
                        previous.clone(),
                        src.clone(),
                        previous,
                        src.clone(),
                    ));
                }
            }
            _ => (),
        }
    }

    pub fn set_type_eq(&mut self, dest: &HirId, src: &HirId) -> Option<()> {
        // we short-circuit if the type is not found
        // this is useful only when some diagnostics has been emited
        // but we want to continue to infer the rest of the types
        let src_t_opt = self.get_type(src);
        if src_t_opt.is_none() {
            error!("Cannot set the type of `{}` to `{}`", dest, src);

            return None;
        }

        let src_t = src_t_opt?.clone();

        self.set_type(dest, &src_t);

        Some(())
    }

    pub fn get_type(&self, hir_id: &HirId) -> Option<&Type> {
        self.get_current_env().and_then(|env| env.get(hir_id))
    }

    pub fn apply_args_type(&mut self, f: &FunctionDecl) {
        f.arguments
            .clone()
            .into_iter()
            .enumerate()
            .for_each(|(i, arg)| {
                self.set_type(
                    &arg.get_hir_id(),
                    &self.current_fn.1.arguments.get(i).unwrap().clone(),
                )
            });
    }

    #[allow(dead_code)]
    pub fn get_fn_types(&self, f: &HirId) -> Option<&HashMap<FuncType, Env>> {
        self.fns.get(f)
    }

    pub fn get_inner(&self) -> &BTreeMap<HirId, HashMap<FuncType, Env>> {
        &self.fns
    }

    #[allow(dead_code)]
    pub fn add_empty(&mut self, hir_id: &HirId) {
        self.fns.entry(hir_id.clone()).or_insert_with(HashMap::new);
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
