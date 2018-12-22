use std::collections::HashMap;

use super::ast::*;
use super::scope::Scopes;

pub struct Context {
    pub scopes: Scopes<TypeInfer>,
    pub cur_type: TypeInfer,
}

impl Context {
    pub fn new() -> Context {
        Context {
            scopes: Scopes::new(),
            cur_type: TypeInfer::Type(None),
        }
    }
}

#[derive(Clone, Debug)]
pub enum TypeInfer {
    FuncType(FuncType),
    Type(Option<Type>),
}

impl TypeInfer {
    pub fn get_ret(&self) -> Option<Type> {
        match self.clone() {
            TypeInfer::FuncType(f) => f.ret,
            TypeInfer::Type(t) => t,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FuncType {
    pub params: HashMap<String, Option<Type>>,
    pub ret: Option<Type>,
    pub solved: bool,
}

impl FuncType {
    pub fn new(func: FunctionDecl) -> FuncType {
        let mut params = HashMap::new();

        for arg in func.arguments {
            params.insert(arg.name, arg.t);
        }

        FuncType {
            solved: params.iter().all(|(_, t)| t.is_some()) && func.t.is_some(),
            params,
            ret: func.t,
        }
    }

    pub fn check_solved(&mut self) {
        self.solved = self.params.iter().all(|(_, t)| t.is_some()) && self.ret.is_some();
    }
}
