use nalgebra::{ClosedAdd, ClosedMul, ClosedSub, Scalar, SimdComplexField};
use num_traits::{One, Zero};

use crate::{Framed, Vector};

pub type Point<Frame, const DIMENSION: usize, T = f32> =
    Framed<Frame, nalgebra::Point<T, DIMENSION>>;
pub type Point2<Frame, T = f32> = Point<Frame, 2, T>;
pub type Point3<Frame, T = f32> = Point<Frame, 3, T>;

#[macro_export]
macro_rules! point {
    ($($parameters:expr),* $(,)?) => {
        linear_algebra::Framed::wrap(nalgebra::point![$($parameters),*])

    };
}

pub fn distance<Frame, const DIMENSION: usize, T>(
    p1: Point<Frame, DIMENSION, T>,
    p2: Point<Frame, DIMENSION, T>,
) -> T::SimdRealField
where
    T: SimdComplexField,
{
    nalgebra::distance(&p1.inner, &p2.inner)
}

pub fn distance_squared<Frame, const DIMENSION: usize, T>(
    p1: Point<Frame, DIMENSION, T>,
    p2: Point<Frame, DIMENSION, T>,
) -> T::SimdRealField
where
    T: SimdComplexField,
{
    nalgebra::distance_squared(&p1.inner, &p2.inner)
}

pub fn center<Frame, const DIMENSION: usize, T>(
    p1: Point<Frame, DIMENSION, T>,
    p2: Point<Frame, DIMENSION, T>,
) -> Point<Frame, DIMENSION, T>
where
    T: SimdComplexField,
{
    Point::wrap(nalgebra::center(&p1.inner, &p2.inner))
}

impl<Frame, const DIMENSION: usize, T> Point<Frame, DIMENSION, T>
where
    T: Scalar + Copy,
{
    pub fn origin() -> Self
    where
        T: Zero,
    {
        Self::wrap(nalgebra::Point::origin())
    }

    pub fn coords(&self) -> Vector<Frame, DIMENSION, T> {
        Framed::wrap(self.inner.coords)
    }

    pub fn map<F, T2>(&self, f: F) -> Point<Frame, DIMENSION, T2>
    where
        T2: Scalar,
        F: FnMut(T) -> T2,
    {
        Framed::wrap(self.inner.map(f))
    }

    pub fn lerp(&self, other: Self, t: T) -> Self
    where
        T: Zero + One + ClosedAdd + ClosedSub + ClosedMul,
    {
        Self::wrap(self.inner.lerp(&other.inner, t))
    }
}

impl<Frame, T> Point2<Frame, T>
where
    T: Scalar + Copy,
{
    pub fn x(&self) -> T {
        self.inner.x
    }

    pub fn y(&self) -> T {
        self.inner.y
    }
}

impl<Frame, T> Point3<Frame, T>
where
    T: Scalar + Copy,
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

    pub fn xy(&self) -> Point2<Frame, T> {
        Point2::wrap(self.inner.xy())
    }

    pub fn xz(&self) -> Point2<Frame, T> {
        Point2::wrap(self.inner.xz())
    }

    pub fn yz(&self) -> Point2<Frame, T> {
        Point2::wrap(self.inner.yz())
    }
}

impl<Frame, const DIMENSION: usize, T> From<nalgebra::SVector<T, DIMENSION>>
    for Point<Frame, DIMENSION, T>
where
    T: Scalar,
{
    fn from(value: nalgebra::SVector<T, DIMENSION>) -> Self {
        Self::wrap(value.into())
    }
}
