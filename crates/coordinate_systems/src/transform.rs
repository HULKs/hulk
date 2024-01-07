use std::{marker::PhantomData, ops::Mul};

use nalgebra::{AbstractRotation, Isometry, SimdRealField};

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
        Self::new(self.inner.clone())
    }
}

impl<From, To, Inner> Copy for Transform<From, To, Inner> where Inner: Copy {}

impl<From, To, Transformer> Transform<From, To, Transformer> {
    pub fn new(inner: Transformer) -> Self {
        Self {
            from: PhantomData,
            to: PhantomData,
            inner,
        }
    }
}

impl<From, To, Transformer, Entity> Mul<Framed<From, Entity>> for Transform<From, To, Transformer>
where
    Transformer: Mul<Entity, Output = Entity>,
{
    type Output = Framed<To, Entity>;

    fn mul(self, rhs: Framed<From, Entity>) -> Self::Output {
        Self::Output::new(self.inner * rhs.inner)
    }
}

impl<From, To, T, R, const D: usize> Transform<From, To, Isometry<T, R, D>>
where
    T::Element: SimdRealField,
    T: SimdRealField,
    R: AbstractRotation<T, D>,
{
    pub fn inverse(&self) -> Transform<To, From, Isometry<T, R, D>> {
        Transform::<To, From, _>::new(self.inner.inverse())
    }
}
