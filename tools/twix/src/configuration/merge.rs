use std::collections::HashMap;
use std::hash::Hash;

pub trait Merge<T> {
    /// Merge `other` into `self`.
    /// `self` acts as a baseline,
    /// and items in `other` take precedence in the case of a conflict.
    fn merge(&mut self, other: T);
}

impl<K, V> Merge<Self> for HashMap<K, V>
where
    K: Eq + Hash,
{
    fn merge(&mut self, other: Self) {
        self.extend(other);
    }
}

impl<T: Merge<T>> Merge<Option<T>> for T {
    fn merge(&mut self, other: Option<T>) {
        if let Some(value) = other {
            self.merge(value);
        }
    }
}
