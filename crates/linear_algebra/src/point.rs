use nalgebra::{ClosedAddAssign, ClosedMulAssign, ClosedSubAssign, Scalar, SimdComplexField};
use num_traits::{One, Zero};
use simba::scalar::SupersetOf;

use crate::{Framed, Vector};

pub type Point<Frame, const DIMENSION: usize, T = f32> =
    Framed<Frame, nalgebra::Point<T, DIMENSION>>;
pub type Point2<Frame, T = f32> = Point<Frame, 2, T>;
pub type Point3<Frame, T = f32> = Point<Frame, 3, T>;

/// Construct a frame-safe point with a frame.
///
/// This macro works like [`nalgebra::point!`], but wraps the result with a frame.
///
/// # Example
/// ```rust
/// use linear_algebra::{point, Point2};
///
/// struct World;
/// let p: Point2<World> = point![1.0, 2.0];
/// ```
#[macro_export]
macro_rules! point {
    (<$frame:ty>, $($parameters:expr),* $(,)?) => {
        $crate::Framed::<$frame, _>::wrap(nalgebra::point![$($parameters),*])
    };
    ($($parameters:expr),* $(,)?) => {
        $crate::Framed::wrap(nalgebra::point![$($parameters),*])
    };
}

/// Computes the distance between two points (wraps [`nalgebra::distance`]).
pub fn distance<Frame, const DIMENSION: usize, T>(
    p1: Point<Frame, DIMENSION, T>,
    p2: Point<Frame, DIMENSION, T>,
) -> T::SimdRealField
where
    T: SimdComplexField,
{
    nalgebra::distance(&p1.inner, &p2.inner)
}

/// Computes the squared distance between two points (wraps [`nalgebra::distance_squared`]).
pub fn distance_squared<Frame, const DIMENSION: usize, T>(
    p1: Point<Frame, DIMENSION, T>,
    p2: Point<Frame, DIMENSION, T>,
) -> T::SimdRealField
where
    T: SimdComplexField,
{
    nalgebra::distance_squared(&p1.inner, &p2.inner)
}

/// Computes the center (midpoint) between two points (wraps [`nalgebra::center`]).
pub fn center<Frame, const DIMENSION: usize, T>(
    p1: Point<Frame, DIMENSION, T>,
    p2: Point<Frame, DIMENSION, T>,
) -> Point<Frame, DIMENSION, T>
where
    T: SimdComplexField,
{
    Point::wrap(nalgebra::center(&p1.inner, &p2.inner))
}

// Any Dimension
impl<Frame, const DIMENSION: usize, T: Scalar> Point<Frame, DIMENSION, T> {
    pub fn origin() -> Self
    where
        T: Zero,
    {
        Self::wrap(nalgebra::Point::origin())
    }

    pub fn cast<T2>(&self) -> Point<Frame, DIMENSION, T2>
    where
        T: Copy,
        T2: Scalar + SupersetOf<T>,
    {
        Framed::wrap(self.inner.cast::<T2>())
    }

    pub fn coords(&self) -> Vector<Frame, DIMENSION, T>
    where
        T: Copy,
    {
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
        T: Zero + One + ClosedAddAssign + ClosedSubAssign + ClosedMulAssign,
    {
        Self::wrap(self.inner.lerp(&other.inner, t))
    }
}

// 2 Dimension
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

    pub fn extend(&self, z: T) -> Point3<Frame, T> {
        Point3::wrap(nalgebra::point![self.x(), self.y(), z,])
    }
}

// 3 Dimension
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
        let point = nalgebra::Point::<T, DIMENSION>::from(value);
        Self::wrap(point)
    }
}
