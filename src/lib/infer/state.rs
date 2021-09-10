use std::{
    collections::{BTreeMap, HashMap},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    ast::Type,
    diagnostics::Diagnostic,
    hir::{HirId, Root},
};

pub type NodeId = u64;

pub type TypeId = u64;

static GLOBAL_NEXT_TYPE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub enum Constraint {
    Eq(TypeId, TypeId),
    Callable(TypeId, TypeId), //(fnType, returnType)
}

#[derive(Debug, Default)]
pub struct InferState {
    node_types: BTreeMap<HirId, TypeId>,
    types: BTreeMap<TypeId, Option<Type>>,
    constraints: Vec<Constraint>,
    pub trait_call_to_mangle: HashMap<HirId, Vec<String>>, // fc_call => prefixes
    pub root: Root,
}

impl InferState {
    pub fn new(root: Root) -> Self {
        Self {
            root,
            ..Self::default()
        }
    }

    pub fn new_type_id(&mut self, hir_id: HirId) -> TypeId {
        let new_type = GLOBAL_NEXT_TYPE_ID.fetch_add(1, Ordering::SeqCst);

        self.types.insert(new_type, None);

        if self.node_types.contains_key(&hir_id) {
            panic!("ALREADY DEFINED TYPE");
        }

        self.node_types.insert(hir_id, new_type);

        new_type
    }

    pub fn new_type_solved(&mut self, hir_id: HirId, t: Type) -> TypeId {
        let new_type = self.new_type_id(hir_id);

        self.types.insert(new_type, Some(t));

        new_type
    }

    pub fn get_type_id(&self, hir_id: HirId) -> Option<TypeId> {
        self.node_types.get(&hir_id).cloned()
    }

    pub fn get_type(&self, t_id: TypeId) -> Option<Type> {
        self.types.get(&t_id).unwrap().clone()
    }

    pub fn get_or_create_type_id_by_type(&mut self, t: &Type) -> Option<TypeId> {
        let mut res = self
            .types
            .iter()
            .find(|(_t_id, t2)| {
                let t2 = t2.as_ref();
                match t2 {
                    Some(t2) => t2 == t,
                    None => false,
                }
            })
            .map(|(t_id, _)| t_id.clone());

        if res.is_none() {
            let new_type = GLOBAL_NEXT_TYPE_ID.fetch_add(1, Ordering::SeqCst);

            self.types.insert(new_type, Some(t.clone()));

            res = Some(new_type)
        }

        res
    }

    pub fn remove_node_id(&mut self, hir_id: HirId) {
        self.node_types.remove(&hir_id);
    }

    pub fn replace_type(&mut self, left: TypeId, right: TypeId) -> Result<bool, Diagnostic> {
        let left_t = self.types.get(&left).unwrap().clone();
        let right_t = self.types.get(&right).unwrap().clone();

        Ok(match (left_t.clone(), right_t.clone()) {
            (Some(_), None) => {
                self.types.insert(right, self.get_type(left));

                true
            }
            (None, Some(_)) => {
                self.types.insert(left, self.get_type(right));

                true
            }

            (Some(Type::FuncType(f)), Some(Type::FuncType(f2))) => {
                // FIXME: Don't rely on names for resolution
                if let Some(Type::FuncType(left_f)) = self.types.get_mut(&left).unwrap() {
                    left_f.name = f2.name.clone();
                }

                self.types.insert(f.ret, self.get_ret(f2.ret));

                f.arguments.iter().enumerate().for_each(|(i, arg)| {
                    self.add_constraint(Constraint::Eq(*arg, *f2.arguments.get(i).unwrap()));
                });

                true
            }
            (Some(left_in), Some(right_in)) => {
                if left_in != right_in {
                    let span = self
                        .node_types
                        .iter()
                        .find(|(_hir_id, t_id)| **t_id == right)
                        .map(|(hir_id, _t_id)| self.root.hir_map.get_node_id(hir_id).unwrap())
                        .map(|node_id| self.root.spans.get(&node_id).unwrap().clone())
                        .unwrap();

                    return Err(Diagnostic::new_type_conflict(
                        span,
                        left_in,
                        right_in,
                        left_t.clone().unwrap(),
                        right_t.clone().unwrap(),
                    ));
                }
                true
            }
            _ => false,
        })
    }

    pub fn replace_type_ret(&mut self, left: TypeId, right: TypeId) -> Result<bool, Diagnostic> {
        let left_t = self.types.get(&left).unwrap().clone();
        let right_t = self.types.get(&right).unwrap().clone();

        Ok(match (left_t.clone(), right_t.clone()) {
            (Some(_), None) => {
                self.types.insert(right, self.get_ret_rec(left));

                true
            }
            (None, Some(_)) => {
                self.types.insert(left, self.get_ret_rec(right));

                true
            }
            (Some(Type::FuncType(_f1)), Some(Type::FuncType(_f2))) => {
                if self.get_ret_rec(left).unwrap() != self.get_ret_rec(right).unwrap() {
                    let span = self
                        .node_types
                        .iter()
                        .find(|(_hir_id, t_id)| **t_id == right)
                        .map(|(hir_id, _t_id)| self.root.hir_map.get_node_id(hir_id).unwrap())
                        .map(|node_id| self.root.spans.get(&node_id).unwrap().clone())
                        .unwrap();

                    return Err(Diagnostic::new_type_conflict(
                        span,
                        self.get_ret_rec(left).unwrap(),
                        self.get_ret_rec(right).unwrap(),
                        left_t.clone().unwrap(),
                        right_t.clone().unwrap(),
                    ));
                } else {
                    true
                }
            }
            (Some(Type::FuncType(_f)), Some(_other)) => {
                if self.get_ret_rec(left).unwrap() != self.get_ret_rec(right).unwrap() {
                    let span = self
                        .node_types
                        .iter()
                        .find(|(_hir_id, t_id)| **t_id == right)
                        .map(|(hir_id, _t_id)| self.root.hir_map.get_node_id(hir_id).unwrap())
                        .map(|node_id| self.root.spans.get(&node_id).unwrap().clone())
                        .unwrap();

                    return Err(Diagnostic::new_type_conflict(
                        span,
                        self.get_ret_rec(left).unwrap(),
                        self.get_ret_rec(right).unwrap(),
                        left_t.clone().unwrap(),
                        right_t.clone().unwrap(),
                    ));
                } else {
                    true
                }
            }
            (Some(_left_in), Some(_)) => {
                // FIXME: this makes the traits and mod tests to fail
                if self.get_ret_rec(left).unwrap() != self.get_ret_rec(right).unwrap() {
                    let span = self
                        .node_types
                        .iter()
                        .find(|(_hir_id, t_id)| **t_id == right)
                        .map(|(hir_id, _t_id)| self.root.hir_map.get_node_id(hir_id).unwrap())
                        .map(|node_id| self.root.spans.get(&node_id).unwrap().clone())
                        .unwrap();

                    return Err(Diagnostic::new_type_conflict(
                        span,
                        self.get_ret_rec(left).unwrap(),
                        self.get_ret_rec(right).unwrap(),
                        left_t.clone().unwrap(),
                        right_t.clone().unwrap(),
                    ));
                } else {
                    // self.types.insert(f.ret, self.get_ret_rec(right));
                }
                true
            }
            (_left_in, _right_in) => true,
        })
    }

    pub fn get_ret(&self, t_id: TypeId) -> Option<Type> {
        let t_id = if let Some(t) = self.get_type(t_id) {
            match t {
                Type::FuncType(f) => f.ret,
                _ => t_id,
            }
        } else {
            t_id
        };

        self.get_type(t_id)
    }

    pub fn get_ret_rec(&self, t_id: TypeId) -> Option<Type> {
        self.get_ret(t_id).and_then(|t| match &t {
            Type::FuncType(f) => self.get_ret_rec(f.ret).or(self.get_ret(f.ret)),
            _ => Some(t),
        })
    }

    pub fn solve_type(&mut self, hir_id: HirId, t: Type) {
        if let Some(t_id) = self.get_type_id(hir_id) {
            if let Some(ref mut t2) = self.types.get_mut(&t_id) {
                **t2 = Some(t)
            }
        }
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        match constraint {
            Constraint::Eq(left, right) => {
                if left == right || left == 0 || right == 0 {
                    return;
                }
            }
            Constraint::Callable(_, _) => (),
        };

        self.constraints.push(constraint);
    }

    pub fn solve(&mut self) -> Result<(), Vec<Diagnostic>> {
        let mut cpy = self.constraints.clone();
        let mut res = vec![];
        let mut diags = vec![];

        let mut i = 0;

        loop {
            for constraint in cpy.clone() {
                match constraint {
                    Constraint::Eq(left, right) => match self.replace_type(left, right) {
                        Ok(replaced) => {
                            if !replaced {
                                res.push(constraint.clone());
                            }
                        }
                        Err(diag) => diags.push(diag),
                    },
                    Constraint::Callable(f, ret) => match self.replace_type_ret(f, ret) {
                        Ok(replaced) => {
                            if !replaced {
                                res.push(constraint.clone());
                            }
                        }
                        Err(diag) => diags.push(diag),
                    },
                }
            }

            i += 1;

            if res.is_empty() || i > 3 {
                break;
            }

            cpy = self.constraints.clone();
            res = vec![];
        }

        if !diags.is_empty() {
            return Err(diags);
        }

        Ok(())
    }

    pub fn get_node_types(&self) -> BTreeMap<HirId, TypeId> {
        self.node_types.clone()
    }

    pub fn get_types(&self) -> (BTreeMap<TypeId, Type>, Vec<Diagnostic>) {
        let mut map = BTreeMap::new();
        let mut diags = vec![];

        for (t_id, t) in &self.types {
            if t.is_none() {
                let span = self
                    .node_types
                    .iter()
                    .find(|(_hir_id, t_id2)| *t_id == **t_id2)
                    .map(|(hir_id, _t_id)| self.root.hir_map.get_node_id(hir_id).unwrap())
                    .map(|node_id| self.root.spans.get(&node_id).unwrap().clone())
                    .unwrap();

                diags.push(Diagnostic::new_unresolved_type(
                    span,
                    *t_id,
                    self.node_types
                        .iter()
                        .find(|(_hir_id, t_id2)| t_id == *t_id2)
                        .map(|x| x.0)
                        .cloned()
                        .unwrap(),
                ));
            } else {
                map.insert(*t_id, t.clone().unwrap());
            }
        }

        (map, diags)
    }
}
