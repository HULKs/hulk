use std::{
    hash::{Hash, Hasher},
    iter::Sum,
    marker::PhantomData,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

/// Tag any value with a coordinate frame.
///
/// This is the core wrapper type for all frame-safe types in this crate.
///
/// # Example
/// ```rust
/// use linear_algebra::{vector, Framed, Vector2};
///
/// let v: nalgebra::Vector2<f32> = nalgebra::vector![1.0, 2.0];
///
/// struct World;
/// let x = Framed::<World, nalgebra::Vector2<f32>>::wrap(v);
/// ```
#[derive(Debug)]
// `repr(transparent)` ensures this struct has the same memory layout as `inner`.
// This guarantees that transmuting between `Framed<Frame, Inner>` and `Inner` is safe.
#[repr(transparent)]
pub struct Framed<Frame, T> {
    frame: PhantomData<Frame>,
    pub inner: T,
}

impl<Frame, T> Clone for Framed<Frame, T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self::wrap(self.inner.clone())
    }
}

impl<Frame, T> Copy for Framed<Frame, T> where T: Copy {}

impl<Frame, T> Default for Framed<Frame, T>
where
    T: Default,
{
    fn default() -> Self {
        Self::wrap(T::default())
    }
}

impl<Frame, T> PartialEq for Framed<Frame, T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<Frame, T> Eq for Framed<Frame, T> where T: Eq {}

impl<Frame, T> Hash for Framed<Frame, T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<Frame, T> Framed<Frame, T> {
    /// Wrap a value in its frame.
    pub const fn wrap(inner: T) -> Self {
        Framed {
            frame: PhantomData,
            inner,
        }
    }
}

// Add
impl<Frame, Lhs, Rhs> Add<Framed<Frame, Rhs>> for Framed<Frame, Lhs>
where
    Lhs: Add<Rhs>,
{
    type Output = Framed<Frame, Lhs::Output>;

    fn add(self, rhs: Framed<Frame, Rhs>) -> Self::Output {
        let inner = self.inner + rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<'lhs, Frame, Lhs, Rhs> Add<Framed<Frame, Rhs>> for &'lhs Framed<Frame, Lhs>
where
    &'lhs Lhs: Add<Rhs>,
{
    type Output = Framed<Frame, <&'lhs Lhs as Add<Rhs>>::Output>;

    fn add(self, rhs: Framed<Frame, Rhs>) -> Self::Output {
        let inner = &self.inner + rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<'rhs, Frame, Lhs, Rhs> Add<&'rhs Framed<Frame, Rhs>> for Framed<Frame, Lhs>
where
    Lhs: Add<&'rhs Rhs>,
{
    type Output = Framed<Frame, <Lhs as Add<&'rhs Rhs>>::Output>;

    fn add(self, rhs: &'rhs Framed<Frame, Rhs>) -> Self::Output {
        let inner = self.inner + &rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<'lhs, 'rhs, Frame, Lhs, Rhs> Add<&'rhs Framed<Frame, Rhs>> for &'lhs Framed<Frame, Lhs>
where
    &'lhs Lhs: Add<&'rhs Rhs>,
{
    type Output = Framed<Frame, <&'lhs Lhs as Add<&'rhs Rhs>>::Output>;

    fn add(self, rhs: &'rhs Framed<Frame, Rhs>) -> Self::Output {
        let inner = &self.inner + &rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<Frame, Lhs, Rhs> AddAssign<Framed<Frame, Rhs>> for Framed<Frame, Lhs>
where
    Lhs: AddAssign<Rhs>,
{
    fn add_assign(&mut self, rhs: Framed<Frame, Rhs>) {
        self.inner += rhs.inner;
    }
}

impl<'rhs, Frame, Lhs, Rhs> AddAssign<&'rhs Framed<Frame, Rhs>> for Framed<Frame, Lhs>
where
    Lhs: AddAssign<&'rhs Rhs>,
{
    fn add_assign(&mut self, rhs: &'rhs Framed<Frame, Rhs>) {
        self.inner += &rhs.inner;
    }
}

// Sub
impl<Frame, Lhs, Rhs> Sub<Framed<Frame, Rhs>> for Framed<Frame, Lhs>
where
    Lhs: Sub<Rhs>,
{
    type Output = Framed<Frame, Lhs::Output>;

    fn sub(self, rhs: Framed<Frame, Rhs>) -> Self::Output {
        let inner = self.inner - rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<'lhs, Frame, Lhs, Rhs> Sub<Framed<Frame, Rhs>> for &'lhs Framed<Frame, Lhs>
where
    &'lhs Lhs: Sub<Rhs>,
{
    type Output = Framed<Frame, <&'lhs Lhs as Sub<Rhs>>::Output>;

    fn sub(self, rhs: Framed<Frame, Rhs>) -> Self::Output {
        let inner = &self.inner - rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<'rhs, Frame, Lhs, Rhs> Sub<&'rhs Framed<Frame, Rhs>> for Framed<Frame, Lhs>
where
    Lhs: Sub<&'rhs Rhs>,
{
    type Output = Framed<Frame, <Lhs as Sub<&'rhs Rhs>>::Output>;

    fn sub(self, rhs: &'rhs Framed<Frame, Rhs>) -> Self::Output {
        let inner = self.inner - &rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<'lhs, 'rhs, Frame, Lhs, Rhs> Sub<&'rhs Framed<Frame, Rhs>> for &'lhs Framed<Frame, Lhs>
where
    &'lhs Lhs: Sub<&'rhs Rhs>,
{
    type Output = Framed<Frame, <&'lhs Lhs as Sub<&'rhs Rhs>>::Output>;

    fn sub(self, rhs: &'rhs Framed<Frame, Rhs>) -> Self::Output {
        let inner = &self.inner - &rhs.inner;
        Self::Output::wrap(inner)
    }
}

impl<Frame, Lhs, Rhs> SubAssign<Framed<Frame, Rhs>> for Framed<Frame, Lhs>
where
    Lhs: SubAssign<Rhs>,
{
    fn sub_assign(&mut self, rhs: Framed<Frame, Rhs>) {
        self.inner -= rhs.inner;
    }
}

impl<'rhs, Frame, Lhs, Rhs> SubAssign<&'rhs Framed<Frame, Rhs>> for Framed<Frame, Lhs>
where
    Lhs: SubAssign<&'rhs Rhs>,
{
    fn sub_assign(&mut self, rhs: &'rhs Framed<Frame, Rhs>) {
        self.inner -= &rhs.inner;
    }
}

// Neg
impl<Frame, T> Neg for Framed<Frame, T>
where
    T: Neg,
{
    type Output = Framed<Frame, T::Output>;

    fn neg(self) -> Self::Output {
        Framed::wrap(-self.inner)
    }
}

// Mul
impl<Frame, Lhs, Rhs> Mul<Rhs> for Framed<Frame, Lhs>
where
    Lhs: Mul<Rhs, Output = Lhs>,
{
    type Output = Framed<Frame, Lhs::Output>;

    fn mul(self, rhs: Rhs) -> Self::Output {
        Self::wrap(self.inner * rhs)
    }
}

impl<Frame, Lhs, Rhs> MulAssign<Rhs> for Framed<Frame, Lhs>
where
    Lhs: MulAssign<Rhs>,
{
    fn mul_assign(&mut self, rhs: Rhs) {
        self.inner *= rhs;
    }
}

// Div
impl<Frame, Lhs, Rhs> Div<Rhs> for Framed<Frame, Lhs>
where
    Lhs: Div<Rhs, Output = Lhs>,
{
    type Output = Framed<Frame, Lhs::Output>;

    fn div(self, rhs: Rhs) -> Self::Output {
        Self::wrap(self.inner / rhs)
    }
}

impl<Frame, Lhs, Rhs> DivAssign<Rhs> for Framed<Frame, Lhs>
where
    Lhs: DivAssign<Rhs>,
{
    fn div_assign(&mut self, rhs: Rhs) {
        self.inner /= rhs;
    }
}

// Sum
impl<Frame, T> Sum for Framed<Frame, T>
where
    T: Sum,
{
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let inner_sum = iter.map(|framed| framed.inner).sum();
        Self::wrap(inner_sum)
    }
}

#[cfg(feature = "serde")]
mod _serde {
    use super::Framed;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl<Frame, T> Serialize for Framed<Frame, T>
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

    impl<'de, Frame, T> Deserialize<'de> for Framed<Frame, T>
    where
        T: Deserialize<'de>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            Ok(Framed::wrap(T::deserialize(deserializer)?))
        }
    }
}

#[cfg(feature = "path_serde")]
mod _path_serde {
    use super::Framed;
    use path_serde::{
        deserialize::{self, PathDeserialize},
        serialize::{self, PathSerialize},
        PathIntrospect,
    };
    use std::collections::HashSet;

    impl<Frame, T> PathSerialize for Framed<Frame, T>
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

    impl<Frame, T> PathDeserialize for Framed<Frame, T>
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

    impl<Frame, T> PathIntrospect for Framed<Frame, T>
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
    use super::Framed;
    use approx::{AbsDiffEq, RelativeEq};

    impl<Frame, T> AbsDiffEq for Framed<Frame, T>
    where
        T: AbsDiffEq,
        Framed<Frame, T>: PartialEq,
    {
        type Epsilon = T::Epsilon;

        fn default_epsilon() -> Self::Epsilon {
            T::default_epsilon()
        }

        fn abs_diff_eq(&self, other: &Self, eps: Self::Epsilon) -> bool {
            self.inner.abs_diff_eq(&other.inner, eps)
        }
    }

    impl<Frame, T> RelativeEq for Framed<Frame, T>
    where
        T: RelativeEq,
        Framed<Frame, T>: PartialEq,
    {
        fn default_max_relative() -> Self::Epsilon {
            T::default_max_relative()
        }

        fn relative_eq(&self, other: &Self, eps: Self::Epsilon, max: Self::Epsilon) -> bool {
            self.inner.relative_eq(&other.inner, eps, max)
        }
    }
}
