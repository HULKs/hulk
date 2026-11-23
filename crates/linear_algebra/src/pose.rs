use nalgebra::SimdRealField;

use crate::{Framed, Isometry2, Isometry3, Orientation2, Orientation3, Point2, Point3};

/// A 2D position and orientation expressed in one reference frame.
pub type Pose2<Frame, T = f32> = Framed<Frame, nalgebra::Isometry2<T>>;
/// A 3D position and orientation expressed in one reference frame.
pub type Pose3<Frame, T = f32> = Framed<Frame, nalgebra::Isometry3<T>>;

// 2 Dimension

impl<Frame, T> Pose2<Frame, T>
where
    T: SimdRealField + Copy,
    T::Element: SimdRealField,
{
    /// Creates a pose from a position and an orientation angle in radians.
    pub fn new(position: Point2<Frame, T>, angle: T) -> Self {
        Self::wrap(nalgebra::Isometry2::new(position.inner.coords, angle))
    }

    pub fn from_parts(position: Point2<Frame, T>, orientation: Orientation2<Frame, T>) -> Self {
        Self::wrap(nalgebra::Isometry2::from_parts(
            position.inner.into(),
            orientation.inner,
        ))
    }

    pub fn zero() -> Self {
        Default::default()
    }

    /// Labels the existing local frame as `From`, preserving its mapping into `Frame`.
    pub fn as_transform<From>(&self) -> Isometry2<From, Frame, T> {
        Isometry2::wrap(self.inner)
    }

    pub fn position(&self) -> Point2<Frame, T> {
        Point2::wrap(self.inner.translation.vector.into())
    }

    pub fn orientation(&self) -> Orientation2<Frame, T>
    where
        T: Copy,
    {
        Orientation2::wrap(self.inner.rotation)
    }

    /// Returns the orientation angle in radians.
    pub fn angle(&self) -> T {
        self.inner.rotation.angle()
    }
}

impl<Frame, T> From<Point2<Frame, T>> for Pose2<Frame, T>
where
    T: SimdRealField,
    T::Element: SimdRealField,
{
    fn from(value: Point2<Frame, T>) -> Self {
        Self::wrap(nalgebra::Isometry2::from(value.inner))
    }
}

// 3 Dimension

impl<Frame, T> Pose3<Frame, T>
where
    T: SimdRealField + Copy,
    T::Element: SimdRealField,
{
    pub fn from_parts(position: Point3<Frame, T>, orientation: Orientation3<Frame, T>) -> Self {
        Self::wrap(nalgebra::Isometry3::from_parts(
            position.inner.into(),
            orientation.inner,
        ))
    }

    /// Labels the existing local frame as `From`, preserving its mapping into `Frame`.
    pub fn as_transform<From>(&self) -> Isometry3<From, Frame, T> {
        Isometry3::wrap(self.inner)
    }

    pub fn position(&self) -> Point3<Frame, T> {
        Point3::wrap(self.inner.translation.vector.into())
    }

    pub fn orientation(&self) -> Orientation3<Frame, T>
    where
        T: Copy,
    {
        Orientation3::wrap(self.inner.rotation)
    }
}

impl<Frame, T> From<Point3<Frame, T>> for Pose3<Frame, T>
where
    T: SimdRealField,
    T::Element: SimdRealField,
{
    fn from(value: Point3<Frame, T>) -> Self {
        Self::wrap(nalgebra::Isometry3::from(value.inner))
    }
}
