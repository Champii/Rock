use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    ast::{Type, TypeSignature},
    diagnostics::Diagnostic,
    hir::*,
};

pub type NodeId = u64;

pub type TypeId = u64;

static GLOBAL_NEXT_TYPE_ID: AtomicU64 = AtomicU64::new(1);

// #[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
// pub enum Constraint {
//     Eq(TypeId, TypeId),
//     Callable(TypeId, TypeId), //(fnType, returnType)
// }

// impl<'a> InferState<'a> {
//     pub fn new(root: &'a Root) -> Self {
//         Self {
//             root,
//             node_types: BTreeMap::new(),
//             types: BTreeMap::new(),
//             constraints: BTreeSet::new(),
//             trait_call_to_mangle: HashMap::new(),
//         }
//     }

//     pub fn is_solved(&self) -> bool {
//         self.types.values().all(Option::is_some)
//     }

//     pub fn new_type_id(&mut self, hir_id: HirId) -> TypeId {
//         if let Some(type_id) = self.node_types.get(&hir_id).cloned() {
//             return type_id;
//         }

//         let new_type = GLOBAL_NEXT_TYPE_ID.fetch_add(1, Ordering::SeqCst);

//         self.types.insert(new_type, None);

//         self.node_types.insert(hir_id, new_type);

//         new_type
//     }

//     pub fn new_type_solved(&mut self, hir_id: HirId, t: Type) -> TypeId {
//         let new_type = self.new_type_id(hir_id);

//         self.types.insert(new_type, Some(t));

//         new_type
//     }

//     pub fn get_type_id(&self, hir_id: HirId) -> Option<TypeId> {
//         self.node_types.get(&hir_id).cloned()
//     }

//     pub fn get_type(&self, t_id: TypeId) -> Option<Type> {
//         self.types.get(&t_id).unwrap().clone()
//     }

//     pub fn get_or_create_type_id_by_type(&mut self, t: &Type) -> Option<TypeId> {
//         let mut res = self
//             .types
//             .iter()
//             .find(|(_t_id, t2)| {
//                 let t2 = t2.as_ref();
//                 match t2 {
//                     Some(t2) => t2 == t,
//                     None => false,
//                 }
//             })
//             .map(|(t_id, _)| t_id.clone());

//         if res.is_none() {
//             let new_type = GLOBAL_NEXT_TYPE_ID.fetch_add(1, Ordering::SeqCst);

//             self.types.insert(new_type, Some(t.clone()));

//             res = Some(new_type)
//         }

//         res
//     }

//     pub fn remove_node_id(&mut self, hir_id: HirId) {
//         self.node_types.remove(&hir_id);
//     }

//     pub fn replace_type(
//         &mut self,
//         left: TypeId,
//         right: TypeId,
//     ) -> Result<Vec<Constraint>, Diagnostic> {
//         let left_t = self.types.get(&left).unwrap().clone();
//         let right_t = self.types.get(&right).unwrap().clone();
//         let res = vec![];

//         Ok(match (left_t.clone(), right_t.clone()) {
//             (Some(_), None) => {
//                 self.types.insert(right, self.get_type(left));

//                 res
//             }
//             (None, Some(_)) => {
//                 self.types.insert(left, self.get_type(right));

//                 res
//             }

//             (Some(Type::FuncType(f)), Some(Type::FuncType(f2))) => {
//                 if f == f2 {
//                     return Ok(res);
//                 }

//                 // // FIXME: Don't rely on names for resolution
//                 if let Some(Type::FuncType(left_f)) = self.types.get_mut(&left).unwrap() {
//                     left_f.name = f2.name.clone();
//                 }

//                 let mut constraints = f
//                     .arguments
//                     .iter()
//                     .enumerate()
//                     .map(|(i, arg)| Constraint::Eq(*arg, *f2.arguments.get(i).unwrap()))
//                     .collect::<Vec<_>>();

//                 constraints.push(Constraint::Eq(f.ret, f2.ret));

//                 constraints
//                     .iter()
//                     .for_each(|constraint| self.add_constraint(constraint.clone()));

//                 constraints
//             }
//             (Some(left_in), Some(right_in)) => {
//                 if left_in != right_in {
//                     let span = self
//                         .node_types
//                         .iter()
//                         .find(|(_hir_id, t_id)| **t_id == right)
//                         .map(|(hir_id, _t_id)| self.root.hir_map.get_node_id(hir_id).unwrap())
//                         .map(|node_id| self.root.spans.get(&node_id).unwrap().clone())
//                         .unwrap();

//                     return Err(Diagnostic::new_type_conflict(
//                         span,
//                         left_in,
//                         right_in,
//                         left_t.clone().unwrap(),
//                         right_t.clone().unwrap(),
//                     ));
//                 }
//                 res
//             }
//             _ => res,
//         })
//     }

//     pub fn replace_type_ret(&mut self, left: TypeId, right: TypeId) -> Result<bool, Diagnostic> {
//         let left_t = self.types.get(&left).unwrap().clone();
//         let right_t = self.types.get(&right).unwrap().clone();

//         Ok(match (left_t.clone(), right_t.clone()) {
//             (Some(_), None) => {
//                 self.types.insert(right, self.get_ret_rec(left));

//                 false
//             }
//             (None, Some(_)) => {
//                 self.types.insert(left, self.get_ret_rec(right));

//                 true
//             }
//             (Some(Type::FuncType(_f1)), Some(Type::FuncType(_f2))) => {
//                 if self.get_ret_rec(left).unwrap() != self.get_ret_rec(right).unwrap() {
//                     let span = self
//                         .node_types
//                         .iter()
//                         .find(|(_hir_id, t_id)| **t_id == right)
//                         .map(|(hir_id, _t_id)| self.root.hir_map.get_node_id(hir_id).unwrap())
//                         .map(|node_id| self.root.spans.get(&node_id).unwrap().clone())
//                         .unwrap();

//                     return Err(Diagnostic::new_type_conflict(
//                         span,
//                         left_t.clone().unwrap(),
//                         right_t.clone().unwrap(),
//                         left_t.clone().unwrap(),
//                         right_t.clone().unwrap(),
//                     ));
//                 } else {
//                     false
//                 }
//                 // false
//             }
//             (Some(Type::FuncType(_f)), Some(_other)) => {
//                 if self.get_ret(left).is_none() {
//                     let span = self
//                         .node_types
//                         .iter()
//                         .find(|(_hir_id, t_id)| **t_id == left)
//                         .map(|(hir_id, _t_id)| self.root.hir_map.get_node_id(hir_id).unwrap())
//                         .map(|node_id| self.root.spans.get(&node_id).unwrap().clone())
//                         .unwrap();

//                     return Err(Diagnostic::new_type_conflict(
//                         span,
//                         left_t.clone().unwrap(),
//                         right_t.clone().unwrap(),
//                         left_t.clone().unwrap(),
//                         right_t.clone().unwrap(),
//                     ));
//                 }
//                 if self.get_ret_rec(left).unwrap() != self.get_ret_rec(right).unwrap() {
//                     let span = self
//                         .node_types
//                         .iter()
//                         .find(|(_hir_id, t_id)| **t_id == left)
//                         .map(|(hir_id, _t_id)| self.root.hir_map.get_node_id(hir_id).unwrap())
//                         .map(|node_id| self.root.spans.get(&node_id).unwrap().clone())
//                         .unwrap();

//                     return Err(Diagnostic::new_type_conflict(
//                         span,
//                         left_t.clone().unwrap(),
//                         right_t.clone().unwrap(),
//                         // self.get_ret_rec(left).unwrap(),
//                         // self.get_ret_rec(right).unwrap(),
//                         left_t.clone().unwrap(),
//                         right_t.clone().unwrap(),
//                     ));
//                 } else {
//                     false
//                 }
//             }
//             (Some(_left_in), Some(Type::FuncType(_f2))) => {
//                 println!("TODO: apply applicative type");

//                 false
//             }
//             (Some(_left_in), Some(_)) => false,
//             (_left_in, _right_in) => false,
//         })
//     }

//     pub fn get_ret(&self, t_id: TypeId) -> Option<Type> {
//         let t_id = if let Some(t) = self.get_type(t_id) {
//             match t {
//                 Type::FuncType(f) => f.ret,
//                 _ => t_id,
//             }
//         } else {
//             t_id
//         };

//         self.get_type(t_id)
//     }

//     pub fn get_ret_rec(&self, t_id: TypeId) -> Option<Type> {
//         self.get_ret(t_id).and_then(|t| match &t {
//             Type::FuncType(f) => {
//                 if f.ret == t_id {
//                     return Some(t);
//                 }

//                 self.get_ret_rec(f.ret).or(self.get_ret(f.ret))
//             }
//             _ => Some(t),
//         })
//     }

//     pub fn solve_type(&mut self, hir_id: HirId, t: Type) {
//         if let Some(t_id) = self.get_type_id(hir_id.clone()) {
//             if let Some(ref mut t2) = self.types.get_mut(&t_id) {
//                 **t2 = Some(t)
//             } else {
//                 panic!("Cannot solve type {:?} {:?}: Cannot get type", hir_id, t);
//             }
//         } else {
//             panic!("Cannot solve type {:?} {:?}: Cannot get type_id", hir_id, t);
//         }
//     }

//     pub fn add_constraint(&mut self, constraint: Constraint) {
//         match constraint {
//             Constraint::Eq(left, right) => {
//                 if left == right || left == 0 || right == 0 {
//                     return;
//                 }
//             }
//             Constraint::Callable(_, _) => (),
//         };

//         self.constraints.insert(constraint);
//     }

//     pub fn solve(&mut self) -> Result<(), Vec<Diagnostic>> {
//         let mut cpy = self.constraints.clone();
//         let mut res = BTreeSet::new();
//         let mut diags = vec![];

//         let mut i = 0;

//         // FIXME: This is a mess
//         loop {
//             for constraint in cpy.clone() {
//                 match constraint {
//                     Constraint::Eq(left, right) => match self.replace_type(left, right) {
//                         Ok(new_constraints) => {
//                             for constraint in new_constraints {
//                                 res.insert(constraint.clone());
//                             }

//                             res.insert(constraint.clone());
//                         }
//                         Err(diag) => {
//                             res.insert(constraint.clone());

//                             if i == 0 {
//                                 diags.push(diag);
//                             }
//                         }
//                     },
//                     Constraint::Callable(f, ret) => match self.replace_type_ret(f, ret) {
//                         Ok(replaced) => {
//                             if !replaced {
//                                 res.insert(constraint.clone());
//                             }
//                         }
//                         Err(diag) => {
//                             res.insert(constraint.clone());

//                             if i == 0 {
//                                 diags.push(diag);
//                             }
//                         }
//                     },
//                 }
//             }

//             i += 1;

//             if res.is_empty() || i > 3 {
//                 break;
//             }

//             cpy = self.constraints.clone();
//             res.clear();
//         }

//         if !diags.is_empty() {
//             return Err(diags);
//         }

//         Ok(())
//     }

//     pub fn get_node_types(&self) -> BTreeMap<HirId, TypeId> {
//         self.node_types.clone()
//     }

//     pub fn get_types(self) -> (BTreeMap<TypeId, Type>, Vec<Diagnostic>) {
//         let mut map = BTreeMap::new();
//         let mut diags = vec![];

//         for (t_id, t) in &self.types {
//             if t.is_none() {
//                 let span = self
//                     .node_types
//                     .iter()
//                     .find(|(_hir_id, t_id2)| *t_id == **t_id2)
//                     .map(|(hir_id, _t_id)| self.root.hir_map.get_node_id(hir_id).unwrap())
//                     .map(|node_id| self.root.spans.get(&node_id).unwrap().clone())
//                     .unwrap();

//                 diags.push(Diagnostic::new_unresolved_type(
//                     span,
//                     *t_id,
//                     self.node_types
//                         .iter()
//                         .find(|(_hir_id, t_id2)| t_id == *t_id2)
//                         .map(|x| x.0)
//                         .cloned()
//                         .unwrap(),
//                 ));
//             } else {
//                 map.insert(*t_id, t.clone().unwrap());
//             }
//         }

//         (map, diags)
//     }
// }

pub type Env = BTreeMap<HirId, Type>;

#[derive(Debug, Default, Clone)]
pub struct Envs {
    fns: BTreeMap<HirId, HashMap<TypeSignature, Env>>,
    current_fn: (HirId, TypeSignature),
}

impl Envs {
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

    pub fn set_current_fn(&mut self, f: (HirId, TypeSignature)) {
        self.fns
            .entry(f.0.clone())
            .or_insert_with(|| vec![(f.1.clone(), Env::default())].into_iter().collect());

        self.current_fn = f;
    }

    pub fn get_current_fn(&self) -> (HirId, TypeSignature) {
        self.current_fn.clone()
    }

    pub fn set_type(&mut self, dest: &HirId, src: &Type) {
        self.get_current_env_mut()
            .unwrap()
            .insert(dest.clone(), src.clone());
    }

    pub fn set_type_eq(&mut self, dest: &HirId, src: &HirId) {
        self.set_type(dest, &self.get_type(src).unwrap().clone());
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
                    self.current_fn.1.args.get(i).unwrap().clone(),
                )
            })
            .collect::<Vec<_>>();

        eq_types.into_iter().for_each(|(t1, t2)| {
            self.get_current_env_mut().unwrap().insert(t1, t2);
        });
    }

    pub fn get_fn_types(&self, f: &HirId) -> Option<&HashMap<TypeSignature, Env>> {
        self.fns.get(f)
    }

    pub fn get_inner(&self) -> &BTreeMap<HirId, HashMap<TypeSignature, Env>> {
        &self.fns
    }
}
