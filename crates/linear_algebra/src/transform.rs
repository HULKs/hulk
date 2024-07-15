use std::{collections::HashSet, marker::PhantomData, ops::Mul};

use approx::{AbsDiffEq, RelativeEq};
use path_serde::{deserialize, serialize, PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::framed::Framed;

#[derive(Debug)]
pub struct Transform<From, To, Inner> {
    from: PhantomData<From>,
    to: PhantomData<To>,
    pub inner: Inner,
}

impl<From, To, Inner> Clone for Transform<From, To, Inner>
where
    Inner: Clone,
{
    fn clone(&self) -> Self {
        Self::wrap(self.inner.clone())
    }
}

impl<From, To, Inner> Copy for Transform<From, To, Inner> where Inner: Copy {}

impl<From, To, Transformer> Transform<From, To, Transformer> {
    pub fn wrap(inner: Transformer) -> Self {
        Self {
            from: PhantomData,
            to: PhantomData,
            inner,
        }
    }
}

impl<From, To, Inner> Default for Transform<From, To, Inner>
where
    Inner: Default,
{
    fn default() -> Self {
        Self::wrap(Inner::default())
    }
}

impl<From, To, Inner> AbsDiffEq for Transform<From, To, Inner>
where
    Transform<From, To, Inner>: PartialEq,
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

impl<From, To, Inner> RelativeEq for Transform<From, To, Inner>
where
    Transform<From, To, Inner>: PartialEq,
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

impl<From, To, Inner> Eq for Transform<From, To, Inner> where Inner: Eq {}

impl<From, To, Transformer, Entity> Mul<Framed<From, Entity>> for Transform<From, To, Transformer>
where
    Transformer: Mul<Entity, Output = Entity>,
{
    type Output = Framed<To, Entity>;

    fn mul(self, rhs: Framed<From, Entity>) -> Self::Output {
        Self::Output::wrap(self.inner * rhs.inner)
    }
}

impl<From, Intermediate, To, Transformer, Inner, Output> Mul<Transform<From, Intermediate, Inner>>
    for Transform<Intermediate, To, Transformer>
where
    Transformer: Mul<Inner, Output = Output>,
{
    type Output = Transform<From, To, Output>;

    fn mul(self, rhs: Transform<From, Intermediate, Inner>) -> Self::Output {
        Self::Output::wrap(self.inner * rhs.inner)
    }
}

impl<From, To, Transformer, Entity> Mul<&Framed<From, Entity>> for Transform<From, To, Transformer>
where
    Transformer: Mul<Entity, Output = Entity>,
    Entity: Copy,
{
    type Output = Framed<To, Entity>;

    fn mul(self, rhs: &Framed<From, Entity>) -> Self::Output {
        Self::Output::wrap(self.inner * rhs.inner)
    }
}

impl<From, To, Transformer, Entity> Mul<Framed<From, Entity>> for &Transform<From, To, Transformer>
where
    Transformer: Mul<Entity, Output = Entity> + Copy,
{
    type Output = Framed<To, Entity>;

    fn mul(self, rhs: Framed<From, Entity>) -> Self::Output {
        Self::Output::wrap(self.inner * rhs.inner)
    }
}

impl<From, To, Inner> PartialEq for Transform<From, To, Inner>
where
    Inner: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<From, To, Inner> Serialize for Transform<From, To, Inner>
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

impl<'a, From, To, Inner> Deserialize<'a> for Transform<From, To, Inner>
where
    Inner: Deserialize<'a>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        Ok(Self::wrap(Inner::deserialize(deserializer)?))
    }
}

impl<From, To, Inner> PathSerialize for Transform<From, To, Inner>
where
    Inner: PathSerialize,
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

impl<From, To, Inner> PathDeserialize for Transform<From, To, Inner>
where
    Inner: PathDeserialize,
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

impl<From, To, Inner> PathIntrospect for Transform<From, To, Inner>
where
    Inner: PathIntrospect,
{
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
        Inner::extend_with_fields(fields, prefix)
    }
}
