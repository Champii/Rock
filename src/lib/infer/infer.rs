use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::ast::{Identity, Type};

pub type NodeId = u64;

pub type TypeId = u64;

static GLOBAL_NEXT_TYPE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct InferState {
    named_types: BTreeMap<String, TypeId>,
    node_types: BTreeMap<NodeId, TypeId>,
    types: BTreeMap<TypeId, Option<Type>>,
}

impl InferState {
    pub fn new() -> Self {
        Self {
            named_types: BTreeMap::new(),
            node_types: BTreeMap::new(),
            types: BTreeMap::new(),
        }
    }

    pub fn new_or_named_type(&mut self, name: String, identity: Identity) -> TypeId {
        match self.named_types.get(&name) {
            Some(t) => {
                self.node_types.insert(identity.node_id, *t);

                *t
            }
            None => {
                let new_type = self.new_type_id(identity);

                self.named_types.insert(name, new_type);

                new_type
            }
        }
    }

    pub fn new_type_id(&mut self, identity: Identity) -> TypeId {
        let new_type = GLOBAL_NEXT_TYPE_ID.fetch_add(1, Ordering::SeqCst);

        self.types.insert(new_type, None);

        self.node_types.insert(identity.node_id, new_type);

        new_type
    }

    pub fn new_type_solved(&mut self, identity: Identity, t: Type) -> TypeId {
        let new_type = self.new_type_id(identity);

        self.types.insert(new_type, Some(t));

        new_type
    }

    pub fn get_type_id(&self, identity: Identity) -> Option<TypeId> {
        self.node_types.get(&identity.node_id).cloned()
    }

    pub fn get_type(&self, t_id: TypeId) -> Option<Type> {
        self.types.get(&t_id).unwrap().clone()
    }

    pub fn get_named_type_id(&self, name: String) -> Option<TypeId> {
        self.named_types.get(&name).cloned()
    }

    pub fn remove_node_id(&mut self, identity: Identity) {
        self.node_types.remove(&identity.node_id);
    }

    pub fn replace_type(&mut self, left: TypeId, right: TypeId) -> bool {
        let left_t = self.types.get(&left).unwrap();
        let right_t = self.types.get(&right).unwrap();

        if right_t.is_none() && left_t.is_some() {
            self.types.insert(right, self.get_ret(left));

            true
        } else if left_t.is_none() && right_t.is_some() {
            self.types.insert(left, self.get_ret(right));

            true
        } else {
            false
        }
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

    pub fn solve_type(&mut self, identity: Identity, t: Type) {
        if let Some(t_id) = self.get_type_id(identity) {
            if let Some(ref mut t2) = self.types.get_mut(&t_id) {
                **t2 = Some(t)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Constraint {
    Eq(TypeId, TypeId),
}

#[derive(Debug)]
pub struct InferBuilder {
    state: InferState,
    constraints: Vec<Constraint>,
}

impl InferBuilder {
    pub fn new(state: InferState) -> Self {
        Self {
            state,
            constraints: vec![],
        }
    }

    pub fn new_named_annotation(&mut self, name: String, identity: Identity) {
        self.state.new_or_named_type(name, identity);
    }

    pub fn new_type_id(&mut self, identity: Identity) {
        self.state.new_type_id(identity);
    }

    pub fn new_type_solved(&mut self, identity: Identity, t: Type) {
        self.state.new_type_solved(identity, t);
    }

    pub fn solve_type(&mut self, identity: Identity, t: Type) {
        self.state.solve_type(identity, t);
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        match constraint {
            Constraint::Eq(left, right) => {
                if left == right || left == 0 || right == 0 {
                    return;
                }
            }
        };

        self.constraints.push(constraint);
    }

    pub fn get_type_id(&self, identity: Identity) -> Option<TypeId> {
        self.state.get_type_id(identity)
    }

    pub fn get_type(&self, t_id: TypeId) -> Option<Type> {
        self.state.get_type(t_id)
    }

    pub fn get_named_type_id(&self, name: String) -> Option<TypeId> {
        self.state.get_named_type_id(name)
    }

    pub fn remove_node_id(&mut self, identity: Identity) {
        self.state.remove_node_id(identity);
    }

    pub fn solve(&mut self) {
        let mut cpy = self.constraints.clone();
        let mut res = vec![];

        let mut i = 0;

        loop {
            for constraint in cpy.clone() {
                match constraint {
                    Constraint::Eq(left, right) => {
                        if !self.state.replace_type(left, right) {
                            res.push(constraint.clone());
                        }
                    }
                }
            }

            i += 1;

            if res.len() == 0 || i > 2 {
                break;
            }

            cpy = self.constraints.clone();
            res = vec![];
        }
    }
}

pub trait Annotate {
    fn annotate(&self, ctx: &mut InferBuilder) {
        self.annotate_primitive(ctx);
    }

    fn annotate_primitive(&self, _ctx: &mut InferBuilder) {
        unimplemented!();
    }
}

#[macro_use]
macro_rules! derive_annotate {
    ($id:tt, $trait:tt, $method:ident, $ctx:tt, [ $($field:ident),* ]) => {
        impl $trait for crate::ast::$id {
            fn $method(&self, ctx: &mut $ctx)  {
                // println!("Annotate: {}", stringify!($id));

                ctx.new_type_id(self.identity.clone());

                $(
                    self.$field.$method(ctx);
                )*
            }
        }
    };
}

#[macro_use]
predef_trait_visitor!(Annotate, annotate, InferBuilder, derive_annotate);

pub trait ConstraintGen {
    fn constrain(&self, ctx: &mut InferBuilder) -> TypeId;
    fn constrain_vec(&self, _ctx: &mut InferBuilder) -> Vec<TypeId> {
        unimplemented!();
    }
}

#[allow(unused_macros)]
#[macro_use]
macro_rules! derive_constraints {
    ($id:tt, $trait:tt, $method:ident, $ctx:tt, [ $($field:ident),* ]) => {
        impl $trait for crate::ast::$id {
            fn $method(&self, ctx: &mut $ctx) -> TypeId{
                // println!("Constraint: {}", stringify!($id));

                $(
                    self.$field.$method(ctx);
                )*
            }
        }
    };
}
