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

    /// Note: Does not check for existing key, replaces old value
    pub fn add(&mut self, s: K, val: T) -> std::result::Result<(), &str> {
        match self.scopes.last_mut().unwrap().insert(s, val) {
            Some(prev) => { Err("Key '{prev}', already exists!") },
            None => { Ok(()) },
        }
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

        assert_eq!(scopes.add("a", 1), Ok(()) );
        assert_eq!(scopes.add("b", 2), Ok(()) );

        assert_eq!(scopes.get("a").unwrap(), 1);
        assert_eq!(scopes.get("b").unwrap(), 2);

        /// Push new scope
        scopes.push();

        assert_eq!(scopes.add("b", 4), Ok(()) );

        assert_eq!(scopes.get("a").unwrap(), 1);
        assert_eq!(scopes.get("b").unwrap(), 4);

        assert_eq!(scopes.add("a", 3), Ok(()) );

        assert_eq!(scopes.get("a").unwrap(), 1);
        assert_eq!(scopes.get("b").unwrap(), 2);

        assert_eq!(scopes.add("a", 4), Err("Key 'a', already exists!"));

        scopes.pop();

        assert_eq!(scopes.get("a").unwrap(), 1);
        assert_eq!(scopes.get("b").unwrap(), 2);
    }
}
