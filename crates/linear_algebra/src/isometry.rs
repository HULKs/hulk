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

    /// Embeds planar translation and yaw in 3D, preserving the source and target frames.
    pub fn to_3d(&self) -> Isometry3<From, To, T> {
        Isometry3::wrap(nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(
                self.inner.translation.x,
                self.inner.translation.y,
                T::zero(),
            ),
            nalgebra::UnitQuaternion::from_axis_angle(
                &nalgebra::Vector3::z_axis(),
                self.inner.rotation.angle(),
            ),
        ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planar_lift_preserves_xy_action_and_height() {
        let planar: Isometry2<(), (), f64> =
            Isometry2::wrap(nalgebra::Isometry2::new(nalgebra::vector![1.0, -2.0], 0.7));
        let point = nalgebra::Point3::new(0.3, -0.4, 0.6);
        let lifted = planar.to_3d().inner * point;
        let projected = planar.inner * nalgebra::Point2::new(point.x, point.y);
        assert!((lifted.xy() - projected).norm() < 1.0e-12);
        assert_eq!(lifted.z, point.z);
    }
}
