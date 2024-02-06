use approx::{AbsDiffEq, RelativeEq};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use std::{
    hash::Hash,
    marker::PhantomData,
    ops::{Add, AddAssign, Div, Mul, MulAssign, Sub, SubAssign},
};

#[derive(Debug)]
pub struct Framed<Frame, Inner> {
    frame: PhantomData<Frame>,
    pub inner: Inner,
}

impl<Frame, Inner> Copy for Framed<Frame, Inner> where Inner: Copy {}

impl<Frame, Inner> Framed<Frame, Inner> {
    pub const fn new(inner: Inner) -> Self {
        Self {
            frame: PhantomData,
            inner,
        }
    }
}

impl<Frame, Inner> Clone for Framed<Frame, Inner>
where
    Inner: Clone,
{
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}

impl<Frame, Inner> Default for Framed<Frame, Inner>
where
    Inner: Default,
{
    fn default() -> Self {
        Self::new(Inner::default())
    }
}

impl<Frame, SelfInner, RightInner> Add<Framed<Frame, RightInner>> for Framed<Frame, SelfInner>
where
    SelfInner: Add<RightInner>,
{
    type Output = Framed<Frame, SelfInner::Output>;

    fn add(self, right: Framed<Frame, RightInner>) -> Self::Output {
        Self::Output::new(self.inner + right.inner)
    }
}

impl<Frame, SelfInner, RightInner> AddAssign<Framed<Frame, RightInner>> for Framed<Frame, SelfInner>
where
    SelfInner: AddAssign<RightInner>,
{
    fn add_assign(&mut self, rhs: Framed<Frame, RightInner>) {
        self.inner += rhs.inner;
    }
}

impl<Frame, SelfInner, RightInner> Sub<Framed<Frame, RightInner>> for Framed<Frame, SelfInner>
where
    SelfInner: Sub<RightInner>,
{
    type Output = Framed<Frame, SelfInner::Output>;

    fn sub(self, right: Framed<Frame, RightInner>) -> Self::Output {
        Self::Output::new(self.inner - right.inner)
    }
}

impl<Frame, SelfInner, RightInner> SubAssign<Framed<Frame, RightInner>> for Framed<Frame, SelfInner>
where
    SelfInner: SubAssign<RightInner>,
{
    fn sub_assign(&mut self, rhs: Framed<Frame, RightInner>) {
        self.inner -= rhs.inner;
    }
}

impl<Frame, Inner> Mul<f32> for Framed<Frame, Inner>
where
    Inner: Mul<f32, Output = Inner>,
{
    type Output = Framed<Frame, Inner::Output>;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.inner * rhs)
    }
}

impl<Frame, Inner> MulAssign<f32> for Framed<Frame, Inner>
where
    Inner: MulAssign<f32>,
{
    fn mul_assign(&mut self, rhs: f32) {
        self.inner *= rhs;
    }
}

impl<Frame, Inner> Div<f32> for Framed<Frame, Inner>
where
    Inner: Div<f32, Output = Inner>,
{
    type Output = Framed<Frame, Inner::Output>;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.inner / rhs)
    }
}

impl<Frame, Inner> Serialize for Framed<Frame, Inner>
where
    Inner: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'a, Frame, Inner> Deserialize<'a> for Framed<Frame, Inner>
where
    Inner: Deserialize<'a>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        Ok(Self::new(Inner::deserialize(deserializer)?))
    }
}

impl<Frame, Inner> SerializeHierarchy for Framed<Frame, Inner>
where
    Inner: SerializeHierarchy,
{
    fn serialize_path<S>(
        &self,
        path: &str,
        serializer: S,
    ) -> Result<S::Ok, serialize_hierarchy::Error<S::Error>>
    where
        S: serde::Serializer,
    {
        self.inner.serialize_path(path, serializer)
    }

    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        deserializer: D,
    ) -> Result<(), serialize_hierarchy::Error<D::Error>>
    where
        D: serde::Deserializer<'de>,
    {
        self.inner.deserialize_path(path, deserializer)
    }

    fn exists(path: &str) -> bool {
        Inner::exists(path)
    }

    fn fill_fields(fields: &mut std::collections::BTreeSet<String>, prefix: &str) {
        Inner::fill_fields(fields, prefix)
    }
}

impl<Frame, Inner> RelativeEq for Framed<Frame, Inner>
where
    Framed<Frame, Inner>: PartialEq,
    Inner: RelativeEq,
{
    fn default_max_relative() -> Self::Epsilon {
        Inner::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        Inner::relative_eq(&self.inner, &other.inner, epsilon, max_relative)
    }
}

impl<Frame, Inner> AbsDiffEq for Framed<Frame, Inner>
where
    Framed<Frame, Inner>: PartialEq,
    Inner: AbsDiffEq,
{
    type Epsilon = Inner::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        Inner::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Inner::abs_diff_eq(&self.inner, &other.inner, epsilon)
    }
}

impl<Frame, Inner> PartialEq for Framed<Frame, Inner>
where
    Inner: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<Frame, Inner> Eq for Framed<Frame, Inner> where Inner: Eq {}

impl<Frame, Inner> Hash for Framed<Frame, Inner>
where
    Inner: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // TODO: do we want to hash the state? does it even do anything?
        self.frame.hash(state);
        self.inner.hash(state);
    }
}
