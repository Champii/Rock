use std::collections::HashMap;
use std::hash::Hash;

pub type Scope<K, T> = HashMap<K, T>;

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
        for scope in self.scopes.iter().rev() {
            if let Some(res) = scope.get(&s) {
                return Some(res.clone());
            }
        }

        None
    }

    pub fn add(&mut self, s: K, val: T) -> Option<T> {
        self.scopes.last_mut().unwrap().insert(s, val)
    }

    pub fn extend(&mut self, other: &Scope<K, T>) {
        self.scopes.last_mut().unwrap().extend(other.clone())
    }

    pub fn push_new(&mut self) {
        self.scopes.push(Scope::new())
    }

    pub fn pop(&mut self) -> Option<HashMap<K, T>> {
        self.scopes.pop()
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

        /// Push new scope
        scopes.push_new();

        scopes.add("b", 4);

        assert_eq!(scopes.get("a").unwrap(), 1);
        assert_eq!(scopes.get("b").unwrap(), 4);

        scopes.add("a", 3);

        assert_eq!(scopes.get("a").unwrap(), 1);
        assert_eq!(scopes.get("b").unwrap(), 2);

        scopes.add("a", 4);

        scopes.pop();

        assert_eq!(scopes.get("a").unwrap(), 1);
        assert_eq!(scopes.get("b").unwrap(), 2);
    }
}
