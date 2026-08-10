//! Lattice traits for dataflow analysis.

use std::collections::HashSet;
use std::hash::Hash;

/// A lattice with join operation for forward dataflow analysis.
pub trait Lattice: Clone {
    /// Join `other` into `self`. Returns `true` if `self` changed.
    fn join(&mut self, other: &Self) -> bool;
}

impl<T: Eq + Hash + Clone> Lattice for HashSet<T> {
    fn join(&mut self, other: &Self) -> bool {
        let mut changed = false;
        for elem in other.iter() {
            if self.insert(elem.clone()) {
                changed = true;
            }
        }
        changed
    }
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone + Eq> Lattice for std::collections::HashMap<K, V> {
    fn join(&mut self, other: &Self) -> bool {
        let mut changed = false;
        for (k, v) in other.iter() {
            if let Some(existing) = self.get(k) {
                if existing != v {
                    // Conflict - in a proper lattice we'd need to handle this
                    // For now, keep the existing value
                }
            } else {
                self.insert(k.clone(), v.clone());
                changed = true;
            }
        }
        changed
    }
}
