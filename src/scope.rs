use llvm::LLVMValue;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Scopes {
    pub scopes: Vec<Scope>,
}

impl Scopes {
    pub fn new() -> Scopes {
        Scopes {
            scopes: vec![Scope::new()],
        }
    }

    pub fn get(&self, s: String) -> Option<*mut LLVMValue> {
        let mut test = self.scopes.clone();
        test.reverse();
        // Here need reverse scopes
        for scope in test {
            if let Some(res) = scope.items.get(&s) {
                return Some(res.clone());
            }
        }

        None
        // self.scopes.last().unwrap()
    }

    pub fn add(&mut self, s: String, val: *mut LLVMValue) {
        // Here need reverse scopes
        let scope = self.scopes.last_mut().unwrap();

        scope.items.insert(s, val);
    }

    pub fn push(&mut self) {
        self.scopes.push(Scope::new())
    }

    pub fn pop(&mut self) {
        self.scopes.pop();
    }
}

#[derive(Clone, Debug)]
pub struct Scope {
    pub items: HashMap<String, *mut LLVMValue>,
}

impl Scope {
    pub fn new() -> Scope {
        Scope {
            items: HashMap::new(),
        }
    }
}
