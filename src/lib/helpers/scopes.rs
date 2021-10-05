use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Debug)]
pub struct Scopes<K, T>
where
    K: Default + Hash + Eq + Clone,
    T: Clone,
{
    pub scopes: Vec<Scope<K, T>>,
}

impl<K, T> Default for Scopes<K, T>
where
    K: Default + Hash + Eq + Clone,
    T: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, T> Scopes<K, T>
where
    K: Default + Hash + Eq + Clone,
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
    }

    pub fn add(&mut self, s: K, val: T) {
        // Here need reverse scopes
        let scope = self.scopes.last_mut().unwrap();

        scope.insert(s, val);
    }

    pub fn push(&mut self) {
        self.scopes.push(Scope::new())
    }

    pub fn pop(&mut self) {
        self.scopes.pop();
    }
}

#[derive(Clone, Debug)]
pub struct Scope<K, T>
where
    K: Default + Hash + Eq + Clone,
    T: Clone,
{
    pub ordering: Vec<K>,
    pub items: HashMap<K, T>,
}

impl<K, T> Default for Scope<K, T>
where
    K: Default + Hash + Eq + Clone,
    T: Clone,
{
    fn default() -> Self {
        Self {
            ordering: Default::default(),
            items: Default::default(),
        }
    }
}

impl<K, T> Scope<K, T>
where
    K: Default + Hash + Eq + Clone,
    T: Clone,
{
    pub fn new() -> Scope<K, T> {
        Default::default()
    }

    pub fn insert(&mut self, s: K, v: T) {
        self.items.insert(s.clone(), v);
        self.ordering.push(s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_scope() {
        let mut scopes = Scopes::default();

        scopes.add("a", 1);
        scopes.add("b", 2);

        assert_eq!(scopes.get("a").unwrap(), 1);
        assert_eq!(scopes.get("b").unwrap(), 2);

        scopes.push();

        scopes.add("a", 3);
        scopes.add("b", 4);

        assert_eq!(scopes.get("a").unwrap(), 3);
        assert_eq!(scopes.get("b").unwrap(), 4);

        scopes.pop();

        assert_eq!(scopes.get("a").unwrap(), 1);
        assert_eq!(scopes.get("b").unwrap(), 2);
    }
}
