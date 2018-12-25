use std::collections::HashMap;

use super::ast::*;
use super::scope::Scopes;

pub struct Symbols {
    pub function_decl: HashMap<String, FunctionDeclMeta>,
}

#[derive(Clone, Debug)]
pub struct FunctionDeclMeta {
    pub params_vec: Vec<Type>,
    pub params: HashMap<String, Option<Type>>,
    pub ret: Option<Type>,
    pub solved: bool,
    pub func: Option<FunctionDecl>,
}

#[derive(Clone, Debug)]
pub struct Context {
    pub calls: HashMap<String, Vec<Vec<TypeInfer>>>,
    pub scopes: Scopes<TypeInfer>,
    pub cur_type: TypeInfer,
}

impl Context {
    pub fn new() -> Context {
        Context {
            calls: HashMap::new(),
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
    pub params_vec: Vec<Type>,
    pub params: HashMap<String, Option<Type>>,
    pub ret: Option<Type>,
    pub solved: bool,
    pub func: FunctionDecl,
}

impl FuncType {
    pub fn new(func: FunctionDecl) -> FuncType {
        let mut params = HashMap::new();
        let mut params_vec = vec![];
        let f = func.clone();

        for arg in func.arguments {
            params.insert(arg.name, arg.t.clone());
            // params_vec.push(arg.t.clone().unwrap());
        }

        FuncType {
            params_vec,
            solved: params.iter().all(|(_, t)| t.is_some()) && func.t.is_some(),
            params,
            ret: func.t,
            func: f,
        }
    }

    pub fn new_from_proto(func: Prototype) -> FuncType {
        let mut params = HashMap::new();
        let f = func.clone();
        let mut params_vec = vec![];

        for arg in func.arguments {
            params_vec.push(arg.clone());
        }

        FuncType {
            solved: true,
            params_vec,
            params,
            ret: Some(func.t),
            func: FunctionDecl {
                name: "".to_string(),
                t: None,
                arguments: vec![],
                body: Body { stmts: vec![] },
            },
        }
    }

    pub fn check_solved(&mut self) {
        self.solved = self.params.iter().all(|(_, t)| t.is_some()) && self.ret.is_some();
    }
}
