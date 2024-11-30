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

impl<T: Merge<O>, O> Merge<Option<O>> for T {
    fn merge(&mut self, other: Option<O>) {
        if let Some(value) = other {
            self.merge(value);
        }
    }
}

macro_rules! impl_merge_as_identity {
    ($ty: ty) => {
        impl Merge<Self> for $ty {
            fn merge(&mut self, other: Self) {
                *self = other;
            }
        }
    };
}

impl_merge_as_identity!(i8);
impl_merge_as_identity!(u8);
impl_merge_as_identity!(i16);
impl_merge_as_identity!(u16);
impl_merge_as_identity!(i32);
impl_merge_as_identity!(u32);
impl_merge_as_identity!(i64);
impl_merge_as_identity!(u64);
impl_merge_as_identity!(isize);
impl_merge_as_identity!(usize);
impl_merge_as_identity!(f32);
impl_merge_as_identity!(f64);
impl_merge_as_identity!(String);
