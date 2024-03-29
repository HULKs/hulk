use nalgebra::{ComplexField, SVector};
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

impl<Frame, const DIMENSION: usize, Scalar> Framed<Frame, SVector<Scalar, DIMENSION>>
where
    Scalar: nalgebra::Scalar + Zero,
{
    pub fn zeros() -> Self {
        Self::wrap(SVector::zeros())
    }
}

impl<Frame, const DIMENSION: usize, Scalar> Framed<Frame, SVector<Scalar, DIMENSION>>
where
    Scalar: nalgebra::Scalar + Copy,
{
    pub fn as_point(&self) -> Point<Frame, DIMENSION, Scalar> {
        Point::from(self.inner)
    }
}

impl<Frame, const DIMENSION: usize, Scalar> Framed<Frame, SVector<Scalar, DIMENSION>>
where
    Scalar: nalgebra::Scalar + ComplexField<RealField = Scalar>,
{
    pub fn normalize(&self) -> Self {
        Self::wrap(self.inner.normalize())
    }

    pub fn try_normalize(&self, min_norm: Scalar) -> Option<Self> {
        Some(Self::wrap(self.inner.try_normalize(min_norm)?))
    }

    pub fn cap_magnitude(&self, max: Scalar) -> Self {
        Self::wrap(self.inner.cap_magnitude(max))
    }

    pub fn unscale(&self, real: Scalar) -> Self {
        Self::wrap(self.inner.unscale(real))
    }

    pub fn norm(&self) -> Scalar {
        self.inner.norm()
    }

    pub fn norm_squared(&self) -> Scalar {
        self.inner.norm_squared()
    }

    pub fn dot(&self, rhs: Self) -> Scalar {
        self.inner.dot(&rhs.inner)
    }

    pub fn angle(&self, rhs: Self) -> Scalar {
        self.inner.angle(&rhs.inner)
    }

    pub fn component_mul(&self, rhs: Self) -> Self {
        Self::wrap(self.inner.component_mul(&rhs.inner))
    }
}

impl<Frame, const DIMENSION: usize, Scalar> Framed<Frame, SVector<Scalar, DIMENSION>>
where
    Scalar: nalgebra::Scalar + ComplexField<RealField = Scalar> + Signed,
{
    pub fn abs(&self) -> Self {
        Self::wrap(self.inner.abs())
    }
}

impl<Frame, Scalar> Framed<Frame, SVector<Scalar, 2>>
where
    Scalar: nalgebra::Scalar + Copy,
{
    pub fn x(&self) -> Scalar {
        self.inner.x
    }

    pub fn y(&self) -> Scalar {
        self.inner.y
    }
}

impl<Frame, Scalar> Framed<Frame, SVector<Scalar, 2>>
where
    Scalar: nalgebra::Scalar + Zero + One + Copy,
{
    pub fn x_axis() -> Self {
        Self::wrap(*SVector::x_axis())
    }

    pub fn y_axis() -> Self {
        Self::wrap(*SVector::y_axis())
    }
}

impl<Frame, Scalar> Framed<Frame, SVector<Scalar, 3>>
where
    Scalar: nalgebra::Scalar + Copy,
{
    pub fn x(&self) -> Scalar {
        self.inner.x
    }

    pub fn y(&self) -> Scalar {
        self.inner.y
    }

    pub fn z(&self) -> Scalar {
        self.inner.z
    }
}

impl<Frame, Scalar> Framed<Frame, SVector<Scalar, 3>>
where
    Scalar: nalgebra::Scalar + Zero + One + Copy,
{
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

impl<Frame, Scalar> Framed<Frame, SVector<Scalar, 3>>
where
    Scalar: nalgebra::Scalar,
{
    pub fn xy(&self) -> Vector2<Frame, Scalar> {
        Vector2::wrap(self.inner.xy())
    }

    pub fn xz(&self) -> Vector2<Frame, Scalar> {
        Vector2::wrap(self.inner.xz())
    }

    pub fn yz(&self) -> Vector2<Frame, Scalar> {
        Vector2::wrap(self.inner.yz())
    }
}
