use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Debug)]
pub struct Scopes<K: Hash + Eq + Clone, T: Clone> {
    pub scopes: Vec<Scope<K, T>>,
}

impl<K, T> Default for Scopes<K, T>
where
    K: Hash + Eq + Clone,
    T: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, T> Scopes<K, T>
where
    K: Hash + Eq + Clone,
    T: Clone,
{
    pub fn new() -> Scopes<K, T> {
        Scopes {
            scopes: vec![Scope::new()],
        }
    }

    pub fn get(&self, s: K) -> Option<T> {
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

    pub fn add(&mut self, s: K, val: T) {
        // Here need reverse scopes
        let scope = self.scopes.last_mut().unwrap();

        scope.insert(s, val);
    }

    pub fn remove(&mut self, s: K) {
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
pub struct Scope<K: Hash + Eq + Clone, T: Clone> {
    pub ordering: Vec<K>,
    pub items: HashMap<K, T>,
}

impl<K, T> Scope<K, T>
where
    K: Hash + Eq + Clone,
    T: Clone,
{
    pub fn new() -> Scope<K, T> {
        Scope {
            ordering: vec![],
            items: HashMap::new(),
        }
    }

    pub fn insert(&mut self, s: K, v: T) {
        self.items.insert(s.clone(), v);
        self.ordering.push(s);
    }

    pub fn remove(&mut self, s: K) {
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
