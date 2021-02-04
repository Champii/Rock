use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Scopes<T: Clone> {
    pub scopes: Vec<Scope<T>>,
}

impl<T: Clone> Default for Scopes<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Scopes<T> {
    pub fn new() -> Scopes<T> {
        Scopes {
            scopes: vec![Scope::new()],
        }
    }

    pub fn get(&self, s: String) -> Option<T> {
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

    pub fn add(&mut self, s: String, val: T) {
        // Here need reverse scopes
        let scope = self.scopes.last_mut().unwrap();

        scope.insert(s, val);
    }

    pub fn remove(&mut self, s: String) {
        // Here need reverse scopes
        let scope = self.scopes.last_mut().unwrap();

        scope.remove(s);
    }

    pub fn push(&mut self) {
        self.scopes.push(Scope::new())
    }

    pub fn pop(&mut self) {
        self.scopes.pop();
    }
}

#[derive(Clone, Debug)]
pub struct Scope<T: Clone> {
    pub ordering: Vec<String>,
    pub items: HashMap<String, T>,
}

impl<T: Clone> Scope<T> {
    pub fn new() -> Scope<T> {
        Scope {
            ordering: vec![],
            items: HashMap::new(),
        }
    }

    pub fn insert(&mut self, s: String, v: T) {
        self.items.insert(s.clone(), v);
        self.ordering.push(s);
    }

    pub fn remove(&mut self, s: String) {
        self.items.remove(&s);
        self.ordering = self.ordering.iter().filter(|x| **x != s).cloned().collect();
    }

    pub fn get_ordered(&self) -> Vec<T> {
        let mut res = vec![];

        for key in &self.ordering {
            res.push(self.items.get(key).unwrap().clone());
        }

        res
    }
}
