use nalgebra::{ComplexField, SVector, Scalar};
use num_traits::{One, Signed, Zero};

use crate::{Framed, Point};

pub type Vector<Frame, const DIMENSION: usize, Scalar = f32> =
    Framed<Frame, nalgebra::SVector<Scalar, DIMENSION>>;
pub type Vector2<Frame, Scalar = f32> = Vector<Frame, 2, Scalar>;
pub type Vector3<Frame, Scalar = f32> = Vector<Frame, 3, Scalar>;

#[macro_export]
macro_rules! vector {
    ($($parameters:expr),* $(,)?) => {
        linear_algebra::Framed::wrap(nalgebra::vector![$($parameters),*])
    };
}

impl<Frame, const DIMENSION: usize, T> Framed<Frame, SVector<T, DIMENSION>>
where
    T: Scalar + ComplexField<RealField = T> + Copy,
{
    pub fn zeros() -> Self
    where
        T: Zero,
    {
        Self::wrap(SVector::zeros())
    }

    pub fn as_point(&self) -> Point<Frame, DIMENSION, T> {
        Point::from(self.inner)
    }

    pub fn normalize(&self) -> Self {
        Self::wrap(self.inner.normalize())
    }

    pub fn try_normalize(&self, min_norm: T) -> Option<Self> {
        Some(Self::wrap(self.inner.try_normalize(min_norm)?))
    }

    pub fn cap_magnitude(&self, max: T) -> Self {
        Self::wrap(self.inner.cap_magnitude(max))
    }

    pub fn unscale(&self, real: T) -> Self {
        Self::wrap(self.inner.unscale(real))
    }

    pub fn norm(&self) -> T {
        self.inner.norm()
    }

    pub fn norm_squared(&self) -> T {
        self.inner.norm_squared()
    }

    pub fn dot(&self, rhs: Self) -> T {
        self.inner.dot(&rhs.inner)
    }

    pub fn angle(&self, rhs: Self) -> T {
        self.inner.angle(&rhs.inner)
    }

    pub fn component_mul(&self, rhs: Self) -> Self {
        Self::wrap(self.inner.component_mul(&rhs.inner))
    }

    pub fn abs(&self) -> Self
    where
        T: Signed,
    {
        Self::wrap(self.inner.abs())
    }
}

impl<Frame, T> Framed<Frame, SVector<T, 2>>
where
    T: Scalar + Zero + One + Copy,
{
    pub fn x(&self) -> T {
        self.inner.x
    }

    pub fn y(&self) -> T {
        self.inner.y
    }

    pub fn x_axis() -> Self {
        Self::wrap(*SVector::x_axis())
    }

    pub fn y_axis() -> Self {
        Self::wrap(*SVector::y_axis())
    }
}

impl<Frame, T> Framed<Frame, SVector<T, 3>>
where
    T: Scalar + Zero + One + Copy,
{
    pub fn x(&self) -> T {
        self.inner.x
    }

    pub fn y(&self) -> T {
        self.inner.y
    }

    pub fn z(&self) -> T {
        self.inner.z
    }

    pub fn xy(&self) -> Vector2<Frame, T> {
        Vector2::wrap(self.inner.xy())
    }

    pub fn xz(&self) -> Vector2<Frame, T> {
        Vector2::wrap(self.inner.xz())
    }

    pub fn yz(&self) -> Vector2<Frame, T> {
        Vector2::wrap(self.inner.yz())
    }

    pub fn x_axis() -> Self {
        Self::wrap(*SVector::x_axis())
    }

    pub fn y_axis() -> Self {
        Self::wrap(*SVector::y_axis())
    }

    pub fn z_axis() -> Self {
        Self::wrap(*SVector::z_axis())
    }
}
