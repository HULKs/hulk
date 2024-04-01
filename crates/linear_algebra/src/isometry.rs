use nalgebra::{AbstractRotation, SimdRealField};

use crate::{
    Orientation2, Orientation3, Point, Point2, Point3, Pose2, Pose3, Rotation3, Transform, Vector2,
    Vector3,
};

pub type Isometry<From, To, const DIMENSION: usize, T, Rotation> =
    Transform<From, To, nalgebra::Isometry<T, Rotation, DIMENSION>>;
pub type Isometry2<From, To, T = f32> = Isometry<From, To, 2, T, nalgebra::UnitComplex<T>>;
pub type Isometry3<From, To, T = f32> = Isometry<From, To, 3, T, nalgebra::UnitQuaternion<T>>;

// Any Dimension

impl<From, To, T, const DIMENSION: usize, Rotation> Isometry<From, To, DIMENSION, T, Rotation>
where
    T::Element: SimdRealField,
    T: SimdRealField,
    Rotation: AbstractRotation<T, DIMENSION>,
{
    pub fn identity() -> Self {
        Self::wrap(nalgebra::Isometry::identity())
    }

    pub fn inverse(&self) -> Transform<To, From, nalgebra::Isometry<T, Rotation, DIMENSION>> {
        Transform::<To, From, _>::wrap(self.inner.inverse())
    }

    pub fn translation(&self) -> Point<To, DIMENSION, T> {
        Point::wrap(self.inner.translation.vector.clone().into())
    }
}

// 2 Dimension

impl<From, To, T> Isometry2<From, To, T>
where
    T::Element: SimdRealField,
    T: SimdRealField + Copy,
{
    pub fn from_parts(translation: Vector2<To, T>, angle: T) -> Self {
        Transform::wrap(nalgebra::Isometry2::new(translation.inner, angle))
    }

    pub fn rotation(angle: T) -> Self {
        Self::wrap(nalgebra::Isometry2::rotation(angle))
    }

    pub fn as_pose(&self) -> Pose2<To, T> {
        Pose2::wrap(self.inner)
    }

    pub fn orientation(&self) -> Orientation2<To, T> {
        Orientation2::wrap(self.inner.rotation)
    }
}

impl<From, To, T> core::convert::From<Vector2<To, T>> for Isometry2<From, To, T>
where
    T::Element: SimdRealField,
    T: SimdRealField + Copy,
{
    fn from(value: Vector2<To, T>) -> Self {
        Self::wrap(nalgebra::Isometry::from(value.inner))
    }
}

impl<From, To, T> core::convert::From<Point2<To, T>> for Isometry2<From, To, T>
where
    T::Element: SimdRealField,
    T: SimdRealField + Copy,
{
    fn from(value: Point2<To, T>) -> Self {
        Self::wrap(nalgebra::Isometry::from(value.inner))
    }
}

// 3 Dimension

impl<From, To, T> Isometry3<From, To, T>
where
    T::Element: SimdRealField,
    T: SimdRealField + Copy,
{
    pub fn from_parts(translation: Vector3<To, T>, orientation: Orientation3<To, T>) -> Self {
        Self::wrap(nalgebra::Isometry3::from_parts(
            translation.inner.into(),
            orientation.inner,
        ))
    }

    pub fn from_rotation(axisangle: Vector3<To, T>) -> Self {
        Self::wrap(nalgebra::Isometry3::rotation(axisangle.inner))
    }

    pub fn from_translation(x: T, y: T, z: T) -> Self {
        Self::wrap(nalgebra::Isometry3::translation(x, y, z))
    }

    pub fn as_pose(&self) -> Pose3<To, T> {
        Pose3::wrap(self.inner)
    }

    pub fn rotation(&self) -> Rotation3<From, To, T> {
        Rotation3::wrap(self.inner.rotation)
    }
}

impl<From, To, T> core::convert::From<Vector3<To, T>> for Isometry3<From, To, T>
where
    T::Element: SimdRealField,
    T: SimdRealField + Copy,
{
    fn from(value: Vector3<To, T>) -> Self {
        Self::wrap(nalgebra::Isometry::from(value.inner))
    }
}

impl<From, To, T> core::convert::From<Point3<To, T>> for Isometry3<From, To, T>
where
    T::Element: SimdRealField,
    T: SimdRealField + Copy,
{
    fn from(value: Point3<To, T>) -> Self {
        Self::wrap(nalgebra::Isometry::from(value.inner))
    }
}

impl<From, To, T> core::convert::From<nalgebra::UnitQuaternion<T>> for Isometry3<From, To, T>
where
    T::Element: SimdRealField,
    T: SimdRealField + Copy,
{
    fn from(value: nalgebra::UnitQuaternion<T>) -> Self {
        Self::wrap(nalgebra::Isometry::from_parts(
            nalgebra::Translation::identity(),
            value,
        ))
    }
}

impl<From, To, T> core::convert::From<Orientation3<To, T>> for Isometry3<From, To, T>
where
    T::Element: SimdRealField,
    T: SimdRealField + Copy,
{
    fn from(value: Orientation3<To, T>) -> Self {
        Self::wrap(nalgebra::Isometry3::from_parts(
            nalgebra::Translation::identity(),
            value.inner,
        ))
    }
}
