use std::{marker::PhantomData, ops::Mul};

use approx::{AbsDiffEq, RelativeEq};
use ros_z::{Message, MessageSchema, SchemaBuilder, SerdeCdrCodec};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::framed::Framed;

/// A coordinate mapping from the source frame `From` to the destination frame `To`.
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
    /// Labels an existing mapping with its source and destination frames without changing it.
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

impl<From, To, Inner> Message for Transform<From, To, Inner>
where
    From: Send + Sync + 'static,
    To: Send + Sync + 'static,
    Inner: Message + Serialize + DeserializeOwned,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        Inner::type_name()
    }
}

impl<From, To, Inner> MessageSchema for Transform<From, To, Inner>
where
    From: Send + Sync + 'static,
    To: Send + Sync + 'static,
    Inner: Message + Serialize + DeserializeOwned,
{
    fn build_schema(
        builder: &mut SchemaBuilder,
    ) -> Result<ros_z_schema::TypeDef, ros_z_schema::SchemaError> {
        Inner::build_schema(builder)
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

impl<From, To, Lhs, Rhs> Mul<Framed<From, Rhs>> for Transform<From, To, Lhs>
where
    Lhs: Mul<Rhs>,
{
    type Output = Framed<To, Lhs::Output>;

    fn mul(self, rhs: Framed<From, Rhs>) -> Self::Output {
        Self::Output::wrap(self.inner * rhs.inner)
    }
}

impl<'lhs, From, To, Lhs, Rhs> Mul<Framed<From, Rhs>> for &'lhs Transform<From, To, Lhs>
where
    &'lhs Lhs: Mul<Rhs>,
{
    type Output = Framed<To, <&'lhs Lhs as Mul<Rhs>>::Output>;

    fn mul(self, rhs: Framed<From, Rhs>) -> Self::Output {
        Self::Output::wrap(&self.inner * rhs.inner)
    }
}

impl<'rhs, From, To, Lhs, Rhs> Mul<&'rhs Framed<From, Rhs>> for Transform<From, To, Lhs>
where
    Lhs: Mul<&'rhs Rhs>,
{
    type Output = Framed<To, <Lhs as Mul<&'rhs Rhs>>::Output>;

    fn mul(self, rhs: &'rhs Framed<From, Rhs>) -> Self::Output {
        Self::Output::wrap(self.inner * &rhs.inner)
    }
}

impl<'lhs, 'rhs, From, To, Lhs, Rhs> Mul<&'rhs Framed<From, Rhs>> for &'lhs Transform<From, To, Lhs>
where
    &'lhs Lhs: Mul<&'rhs Rhs>,
{
    type Output = Framed<To, <&'lhs Lhs as Mul<&'rhs Rhs>>::Output>;

    fn mul(self, rhs: &'rhs Framed<From, Rhs>) -> Self::Output {
        Self::Output::wrap(&self.inner * &rhs.inner)
    }
}

impl<From, Mid, To, Lhs, Rhs> Mul<Transform<From, Mid, Rhs>> for Transform<Mid, To, Lhs>
where
    Lhs: Mul<Rhs>,
{
    type Output = Transform<From, To, Lhs::Output>;

    fn mul(self, rhs: Transform<From, Mid, Rhs>) -> Self::Output {
        Self::Output::wrap(self.inner * rhs.inner)
    }
}

impl<'lhs, From, Mid, To, Lhs, Rhs> Mul<Transform<From, Mid, Rhs>> for &'lhs Transform<Mid, To, Lhs>
where
    &'lhs Lhs: Mul<Rhs>,
{
    type Output = Transform<From, To, <&'lhs Lhs as Mul<Rhs>>::Output>;

    fn mul(self, rhs: Transform<From, Mid, Rhs>) -> Self::Output {
        Self::Output::wrap(&self.inner * rhs.inner)
    }
}

impl<'rhs, From, Mid, To, Lhs, Rhs> Mul<&'rhs Transform<From, Mid, Rhs>> for Transform<Mid, To, Lhs>
where
    Lhs: Mul<&'rhs Rhs>,
{
    type Output = Transform<From, To, <Lhs as Mul<&'rhs Rhs>>::Output>;

    fn mul(self, rhs: &'rhs Transform<From, Mid, Rhs>) -> Self::Output {
        Self::Output::wrap(self.inner * &rhs.inner)
    }
}

impl<'lhs, 'rhs, From, Mid, To, Lhs, Rhs> Mul<&'rhs Transform<From, Mid, Rhs>>
    for &'lhs Transform<Mid, To, Lhs>
where
    &'lhs Lhs: Mul<&'rhs Rhs>,
{
    type Output = Transform<From, To, <&'lhs Lhs as Mul<&'rhs Rhs>>::Output>;

    fn mul(self, rhs: &'rhs Transform<From, Mid, Rhs>) -> Self::Output {
        Self::Output::wrap(&self.inner * &rhs.inner)
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

#[cfg(test)]
mod tests {
    use nalgebra::{DMatrix, SMatrix};

    use crate::{Transform, Vector3, vector};

    #[derive(Debug)]
    struct A;
    #[derive(Debug)]
    struct B;
    #[derive(Debug)]
    struct C;

    #[test]
    fn compose_borrowed_dynamic_matrices_without_consuming_them() {
        let a_to_b = Transform::<A, B, _>::wrap(DMatrix::from_row_slice(2, 2, &[1, 2, 0, 1]));
        let b_to_c = Transform::<B, C, _>::wrap(DMatrix::from_row_slice(2, 2, &[2, 0, 0, 3]));

        let a_to_c: Transform<A, C, _> = &b_to_c * &a_to_b;

        assert_eq!(a_to_c.inner, DMatrix::from_row_slice(2, 2, &[2, 4, 0, 3]));
        assert_eq!(b_to_c.clone() * &a_to_b, a_to_c);
        assert_eq!(&b_to_c * a_to_b, a_to_c);
    }

    #[test]
    fn rectangular_transform_changes_the_vector_dimension() {
        let a_to_b =
            Transform::<A, B, _>::wrap(SMatrix::<i32, 3, 2>::from_row_slice(&[1, 0, 0, 1, 2, 3]));

        let output: Vector3<B, _> = a_to_b * vector![<A>, 2, 3];

        assert_eq!(output, vector![<B>, 2, 3, 13]);
    }
}
