use nalgebra::{AbstractRotation, SimdRealField};

use crate::{
    Orientation2, Orientation3, Point, Point2, Point3, Pose2, Pose3, Rotation2, Rotation3,
    Transform, Vector2, Vector3,
};

pub type Isometry<From, To, const DIMENSION: usize, T, Rotation> =
    Transform<From, To, nalgebra::Isometry<T, Rotation, DIMENSION>>;
pub type Isometry2<From, To, T = f32> = Isometry<From, To, 2, T, nalgebra::UnitComplex<T>>;
pub type Isometry3<From, To, T = f32> = Isometry<From, To, 3, T, nalgebra::UnitQuaternion<T>>;

impl<From, To, T, const DIMENSION: usize, Rotation> Isometry<From, To, DIMENSION, T, Rotation>
where
    T::Element: SimdRealField,
    T: SimdRealField,
    Rotation: AbstractRotation<T, DIMENSION>,
{
    /// Returns the identity transform.
    pub fn identity() -> Self {
        Self::wrap(nalgebra::Isometry::identity())
    }

    /// Returns the inverse transform, swapping source and destination frames.
    pub fn inverse(&self) -> Transform<To, From, nalgebra::Isometry<T, Rotation, DIMENSION>> {
        Transform::<To, From, _>::wrap(self.inner.inverse())
    }

    /// Returns the translation component expressed in the destination frame.
    pub fn translation(&self) -> Point<To, DIMENSION, T> {
        Point::wrap(self.inner.translation.vector.clone().into())
    }
}

impl<From, To, T> Isometry2<From, To, T>
where
    T::Element: SimdRealField,
    T: SimdRealField + Copy,
{
    /// Creates an isometry from a translation and angle in radians.
    pub fn new(translation: Vector2<To, T>, angle: T) -> Self {
        Self::wrap(nalgebra::Isometry2::new(translation.inner, angle))
    }

    /// Creates an isometry from a translation and orientation in the destination frame.
    pub fn from_parts(translation: Vector2<To, T>, orientation: Orientation2<To, T>) -> Self {
        Self::wrap(nalgebra::Isometry2::from_parts(
            translation.inner.into(),
            orientation.inner,
        ))
    }

    /// Creates a pure rotation isometry with zero translation.
    pub fn from_angle(angle: T) -> Self {
        Self::wrap(nalgebra::Isometry2::rotation(angle))
    }

    /// Converts this isometry (transform) into a pose in the destination frame.
    ///
    /// # Example
    /// ```
    /// use linear_algebra::{vector, Isometry2, Pose2};
    ///
    /// struct Robot;
    /// struct Ground;
    /// let robot_to_ground: Isometry2<Robot, Ground> = Isometry2::new(vector![1.0, 2.0], 0.5);
    /// let robot: Pose2<Ground> = robot_to_ground.as_pose();
    /// ```
    pub fn as_pose(&self) -> Pose2<To, T> {
        Pose2::wrap(self.inner)
    }

    /// Returns the rotation component as a frame-safe transform.
    pub fn rotation(&self) -> Rotation2<From, To, T> {
        Rotation2::wrap(self.inner.rotation)
    }

    /// Returns the scalar rotation angle in radians.
    pub fn angle(&self) -> T {
        self.inner.rotation.angle()
    }

    /// Returns the orientation component in the destination frame.
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

impl<From, To, T> Isometry3<From, To, T>
where
    T::Element: SimdRealField,
    T: SimdRealField + Copy,
{
    /// Creates an isometry from a translation and orientation in the destination frame.
    pub fn from_parts(translation: Vector3<To, T>, orientation: Orientation3<To, T>) -> Self {
        Self::wrap(nalgebra::Isometry3::from_parts(
            translation.inner.into(),
            orientation.inner,
        ))
    }

    /// Creates a pure rotation isometry with zero translation.
    pub fn from_axis_angle(axis_angle: Vector3<To, T>) -> Self {
        Self::wrap(nalgebra::Isometry3::rotation(axis_angle.inner))
    }

    /// Creates a pure translation isometry.
    pub fn from_translation(translation: Vector3<To, T>) -> Self {
        translation.into()
    }

    /// Converts this isometry (transform) into a pose in the destination frame.
    ///
    /// # Example
    /// ```
    /// use linear_algebra::{vector, Isometry3, Orientation3, Pose3};
    ///
    /// struct Robot;
    /// struct Ground;
    /// let robot_to_ground: Isometry3<Robot, Ground> =
    ///     Isometry3::from_parts(vector![1.0, 2.0, 3.0], Orientation3::identity());
    /// let robot: Pose3<Ground> = robot_to_ground.as_pose();
    /// ```
    pub fn as_pose(&self) -> Pose3<To, T> {
        Pose3::wrap(self.inner)
    }

    /// Returns the rotation component as a frame-safe transform.
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
    use crate::{point, vector};

    #[derive(Debug)]
    struct Robot;
    #[derive(Debug)]
    struct Field;

    #[test]
    fn pure_rotation_isometry2_exposes_rotation_and_angle_on_self() {
        let isometry = Isometry2::<Robot, Field>::from_angle(0.5);

        assert!((isometry.angle() - 0.5).abs() < 1.0e-6);
        assert!((isometry.rotation().angle() - 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn pure_translation_isometry3_can_be_created_from_framed_vector() {
        let isometry = Isometry3::<Robot, Field>::from_translation(vector![1.0, 2.0, 3.0]);

        assert_eq!(isometry.translation(), point![1.0, 2.0, 3.0]);
    }
}
