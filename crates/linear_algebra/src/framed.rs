use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    iter::Sum,
    marker::PhantomData,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use approx::{AbsDiffEq, RelativeEq};
use num_traits::Num;
use serde::{Deserialize, Serialize};

use path_serde::{deserialize, serialize, PathDeserialize, PathIntrospect, PathSerialize};

#[derive(Debug)]
pub struct Framed<Frame, Inner> {
    frame: PhantomData<Frame>,
    pub inner: Inner,
}

impl<Frame, Inner> Copy for Framed<Frame, Inner> where Inner: Copy {}

impl<Frame, Inner> Framed<Frame, Inner> {
    pub const fn wrap(inner: Inner) -> Self {
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
        Self::wrap(self.inner.clone())
    }
}

impl<Frame, Inner> Default for Framed<Frame, Inner>
where
    Inner: Default,
{
    fn default() -> Self {
        Self::wrap(Inner::default())
    }
}

impl<Frame, SelfInner, RightInner> Add<Framed<Frame, RightInner>> for Framed<Frame, SelfInner>
where
    SelfInner: Add<RightInner>,
{
    type Output = Framed<Frame, SelfInner::Output>;

    fn add(self, right: Framed<Frame, RightInner>) -> Self::Output {
        Self::Output::wrap(self.inner + right.inner)
    }
}

impl<'lhs, Frame, SelfInner, RightInner> Add<Framed<Frame, RightInner>>
    for &'lhs Framed<Frame, SelfInner>
where
    &'lhs SelfInner: Add<RightInner>,
{
    type Output = Framed<Frame, <&'lhs SelfInner as Add<RightInner>>::Output>;

    fn add(self, right: Framed<Frame, RightInner>) -> Self::Output {
        Self::Output::wrap(&self.inner + right.inner)
    }
}

impl<'rhs, Frame, SelfInner, RightInner> Add<&'rhs Framed<Frame, RightInner>>
    for Framed<Frame, SelfInner>
where
    SelfInner: Add<&'rhs RightInner>,
{
    type Output = Framed<Frame, <SelfInner as Add<&'rhs RightInner>>::Output>;

    fn add(self, right: &'rhs Framed<Frame, RightInner>) -> Self::Output {
        Self::Output::wrap(self.inner + &right.inner)
    }
}

impl<'lhs, 'rhs, Frame, SelfInner, RightInner> Add<&'rhs Framed<Frame, RightInner>>
    for &'lhs Framed<Frame, SelfInner>
where
    &'lhs SelfInner: Add<&'rhs RightInner>,
{
    type Output = Framed<Frame, <&'lhs SelfInner as Add<&'rhs RightInner>>::Output>;

    fn add(self, right: &'rhs Framed<Frame, RightInner>) -> Self::Output {
        Self::Output::wrap(&self.inner + &right.inner)
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

impl<'rhs, Frame, SelfInner, RightInner> AddAssign<&'rhs Framed<Frame, RightInner>>
    for Framed<Frame, SelfInner>
where
    SelfInner: AddAssign<&'rhs RightInner>,
{
    fn add_assign(&mut self, right: &'rhs Framed<Frame, RightInner>) {
        self.inner += &right.inner
    }
}

impl<Frame, SelfInner, RightInner> Sub<Framed<Frame, RightInner>> for Framed<Frame, SelfInner>
where
    SelfInner: Sub<RightInner>,
{
    type Output = Framed<Frame, SelfInner::Output>;

    fn sub(self, right: Framed<Frame, RightInner>) -> Self::Output {
        Self::Output::wrap(self.inner - right.inner)
    }
}

impl<'lhs, Frame, SelfInner, RightInner> Sub<Framed<Frame, RightInner>>
    for &'lhs Framed<Frame, SelfInner>
where
    &'lhs SelfInner: Sub<RightInner>,
{
    type Output = Framed<Frame, <&'lhs SelfInner as Sub<RightInner>>::Output>;

    fn sub(self, right: Framed<Frame, RightInner>) -> Self::Output {
        Self::Output::wrap(&self.inner - right.inner)
    }
}

impl<'rhs, Frame, SelfInner, RightInner> Sub<&'rhs Framed<Frame, RightInner>>
    for Framed<Frame, SelfInner>
where
    SelfInner: Sub<&'rhs RightInner>,
{
    type Output = Framed<Frame, <SelfInner as Sub<&'rhs RightInner>>::Output>;

    fn sub(self, right: &'rhs Framed<Frame, RightInner>) -> Self::Output {
        Self::Output::wrap(self.inner - &right.inner)
    }
}

impl<'lhs, 'rhs, Frame, SelfInner, RightInner> Sub<&'rhs Framed<Frame, RightInner>>
    for &'lhs Framed<Frame, SelfInner>
where
    &'lhs SelfInner: Sub<&'rhs RightInner>,
{
    type Output = Framed<Frame, <&'lhs SelfInner as Sub<&'rhs RightInner>>::Output>;

    fn sub(self, right: &'rhs Framed<Frame, RightInner>) -> Self::Output {
        Self::Output::wrap(&self.inner - &right.inner)
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

impl<'rhs, Frame, SelfInner, RightInner> SubAssign<&'rhs Framed<Frame, RightInner>>
    for Framed<Frame, SelfInner>
where
    SelfInner: SubAssign<&'rhs RightInner>,
{
    fn sub_assign(&mut self, right: &'rhs Framed<Frame, RightInner>) {
        self.inner -= &right.inner
    }
}

impl<Frame, Inner> Neg for Framed<Frame, Inner>
where
    Inner: Neg,
{
    type Output = Framed<Frame, Inner::Output>;

    fn neg(self) -> Self::Output {
        Framed::wrap(self.inner.neg())
    }
}

impl<Frame, Inner, T> Mul<T> for Framed<Frame, Inner>
where
    Inner: Mul<T, Output = Inner>,
    T: Num,
{
    type Output = Framed<Frame, Inner::Output>;

    fn mul(self, rhs: T) -> Self::Output {
        Self::wrap(self.inner * rhs)
    }
}

impl<Frame, Inner, T> MulAssign<T> for Framed<Frame, Inner>
where
    Inner: MulAssign<T>,
    T: Num,
{
    fn mul_assign(&mut self, rhs: T) {
        self.inner *= rhs;
    }
}

impl<Frame, Inner, T> Div<T> for Framed<Frame, Inner>
where
    Inner: Div<T, Output = Inner>,
    T: Num,
{
    type Output = Framed<Frame, Inner::Output>;

    fn div(self, rhs: T) -> Self::Output {
        Self::wrap(self.inner / rhs)
    }
}

impl<Frame, Inner, T> DivAssign<T> for Framed<Frame, Inner>
where
    Inner: DivAssign<T>,
    T: Num,
{
    fn div_assign(&mut self, rhs: T) {
        self.inner /= rhs;
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
        Ok(Self::wrap(Inner::deserialize(deserializer)?))
    }
}

impl<Frame, Inner> PathSerialize for Framed<Frame, Inner>
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

impl<Frame, Inner> PathDeserialize for Framed<Frame, Inner>
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

impl<Frame, Inner> PathIntrospect for Framed<Frame, Inner>
where
    Inner: PathIntrospect,
{
    fn extend_with_fields(fields: &mut HashSet<String>, prefix: &str) {
        Inner::extend_with_fields(fields, prefix)
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
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<Frame, Inner> Sum for Framed<Frame, Inner>
where
    Inner: Sum,
{
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::wrap(iter.map(|framed| framed.inner).sum())
    }
}

// TODO: Fix and reduce trait bounds and remove Point::cast
// impl<T, R: Dim, C: Dim, S: RawStorage<T, R, C>, Frame> Framed<Frame, Matrix<T, R, C, S>> {
//     pub fn cast<T2>(&self) -> Framed<Frame, Matrix<T2, R, C, S>>
//     where
//         T: Scalar,
//         T2: Scalar + SupersetOf<T>,
//         Matrix<T, R, C, S>: SubsetOf<Matrix<T2, R, C, S>>,
//         Matrix<T2, R, C, S>: SupersetOf<Matrix<T, R, C, S>>,
//         DefaultAllocator: Allocator<T2, R, C>,
//     {
//         Framed::wrap(self.inner.cast::<T2>())
//     }
// }
