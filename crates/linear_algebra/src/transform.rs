use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Mul,
};

use crate::framed::Framed;

/// Tag any value as a transform between two coordinate frames.
///
/// This is the core wrapper type for all coordinate-safe transforms in this crate. It encodes both
/// the source `From` and destination `To` frames at the type level, ensuring that only compatible
/// operations are allowed.
///
/// # Example
/// ```rust
/// use linear_algebra::{Transform, Vector2};
///
/// struct A;
/// struct B;
/// let t: Transform<A, B, nalgebra::Matrix2<f32>> = Transform::wrap(nalgebra::Matrix2::identity());
/// ```
#[derive(Debug)]
pub struct Transform<From, To, T> {
    from: PhantomData<From>,
    to: PhantomData<To>,
    pub inner: T,
}

impl<From, To, T> Clone for Transform<From, To, T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self::wrap(self.inner.clone())
    }
}

impl<From, To, T> Copy for Transform<From, To, T> where T: Copy {}

impl<From, To, T> Default for Transform<From, To, T>
where
    T: Default,
{
    fn default() -> Self {
        Self::wrap(T::default())
    }
}

impl<From, To, T> PartialEq for Transform<From, To, T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<From, To, Inner> Eq for Transform<From, To, Inner> where Inner: Eq {}

impl<From, To, T> Hash for Transform<From, To, T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<From, To, T> Transform<From, To, T> {
    /// Wrap a value in its frame.
    pub fn wrap(inner: T) -> Self {
        Self {
            from: PhantomData,
            to: PhantomData,
            inner,
        }
    }
}

impl<From, To, T, U> Mul<Framed<From, U>> for Transform<From, To, T>
where
    T: Mul<U>,
{
    type Output = Framed<To, T::Output>;

    fn mul(self, rhs: Framed<From, U>) -> Self::Output {
        let inner = self.inner * rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<'lhs, From, To, T, U> Mul<Framed<From, U>> for &'lhs Transform<From, To, T>
where
    &'lhs T: Mul<U>,
{
    type Output = Framed<To, <&'lhs T as Mul<U>>::Output>;

    fn mul(self, rhs: Framed<From, U>) -> Self::Output {
        let inner = &self.inner * rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<'rhs, From, To, T, U> Mul<&'rhs Framed<From, U>> for Transform<From, To, T>
where
    T: Mul<&'rhs U>,
{
    type Output = Framed<To, T::Output>;

    fn mul(self, rhs: &'rhs Framed<From, U>) -> Self::Output {
        let inner = self.inner * &rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<'lhs, 'rhs, From, To, T, U> Mul<&'rhs Framed<From, U>> for &'lhs Transform<From, To, T>
where
    &'lhs T: Mul<&'rhs U>,
{
    type Output = Framed<To, <&'lhs T as Mul<&'rhs U>>::Output>;

    fn mul(self, rhs: &'rhs Framed<From, U>) -> Self::Output {
        let inner = &self.inner * &rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<From, Mid, To, T, U> Mul<Transform<From, Mid, U>> for Transform<Mid, To, T>
where
    T: Mul<U>,
{
    type Output = Transform<From, To, T::Output>;

    fn mul(self, rhs: Transform<From, Mid, U>) -> Self::Output {
        let inner = self.inner * rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<'lhs, From, Mid, To, T, U> Mul<Transform<From, Mid, U>> for &'lhs Transform<Mid, To, T>
where
    &'lhs T: Mul<U>,
{
    type Output = Transform<From, To, <&'lhs T as Mul<U>>::Output>;

    fn mul(self, rhs: Transform<From, Mid, U>) -> Self::Output {
        let inner = &self.inner * rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<'rhs, From, Mid, To, T, U> Mul<&'rhs Transform<From, Mid, U>> for Transform<Mid, To, T>
where
    T: Mul<&'rhs U>,
{
    type Output = Transform<From, To, <T as Mul<&'rhs U>>::Output>;

    fn mul(self, rhs: &'rhs Transform<From, Mid, U>) -> Self::Output {
        let inner = self.inner * &rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<'lhs, 'rhs, From, Mid, To, T, U> Mul<&'rhs Transform<From, Mid, U>>
    for &'lhs Transform<Mid, To, T>
where
    &'lhs T: Mul<&'rhs U>,
{
    type Output = Transform<From, To, <&'lhs T as Mul<&'rhs U>>::Output>;

    fn mul(self, rhs: &'rhs Transform<From, Mid, U>) -> Self::Output {
        let inner = &self.inner * &rhs.inner;
        Self::Output::wrap(inner)
    }
}

#[cfg(feature = "serde")]
mod _serde {
    use super::Transform;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl<From, To, T> Serialize for Transform<From, To, T>
    where
        T: Serialize,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            self.inner.serialize(serializer)
        }
    }

    impl<'a, From, To, Inner> Deserialize<'a> for Transform<From, To, Inner>
    where
        Inner: Deserialize<'a>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'a>,
        {
            Ok(Self::wrap(Inner::deserialize(deserializer)?))
        }
    }
}

#[cfg(feature = "path_serde")]
mod _path_serde {
    use super::Transform;
    use path_serde::{
        deserialize::{self, PathDeserialize},
        serialize::{self, PathSerialize},
        PathIntrospect,
    };
    use std::collections::HashSet;

    impl<From, To, T> PathSerialize for Transform<From, To, T>
    where
        T: PathSerialize,
    {
        fn serialize_path<S>(
            &self,
            path: &str,
            serializer: S,
        ) -> Result<S::Ok, serialize::Error<S::Error>>
        where
            S: serde::Serializer,
        {
            self.inner.serialize_path(path, serializer)
        }
    }

    impl<From, To, T> PathDeserialize for Transform<From, To, T>
    where
        T: PathDeserialize,
    {
        fn deserialize_path<'de, D>(
            &mut self,
            path: &str,
            deserializer: D,
        ) -> Result<(), deserialize::Error<D::Error>>
        where
            D: serde::Deserializer<'de>,
        {
            self.inner.deserialize_path(path, deserializer)
        }
    }

    impl<From, To, T> PathIntrospect for Transform<From, To, T>
    where
        T: PathIntrospect,
    {
        fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
            T::extend_with_fields(fields, prefix)
        }
    }
}

#[cfg(feature = "approx")]
mod _approx {
    use super::Transform;
    use approx::{AbsDiffEq, RelativeEq};

    impl<From, To, T> AbsDiffEq for Transform<From, To, T>
    where
        Transform<From, To, T>: PartialEq,
        T: AbsDiffEq,
    {
        type Epsilon = T::Epsilon;

        fn default_epsilon() -> Self::Epsilon {
            T::default_epsilon()
        }

        fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
            self.inner.abs_diff_eq(&other.inner, epsilon)
        }
    }

    impl<From, To, T> RelativeEq for Transform<From, To, T>
    where
        Transform<From, To, T>: PartialEq,
        T: RelativeEq,
    {
        fn default_max_relative() -> Self::Epsilon {
            T::default_max_relative()
        }

        fn relative_eq(
            &self,
            other: &Self,
            epsilon: Self::Epsilon,
            max_relative: Self::Epsilon,
        ) -> bool {
            self.inner.relative_eq(&other.inner, epsilon, max_relative)
        }
    }
}
