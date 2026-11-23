use nalgebra::{AbstractRotation, SimdRealField};

use crate::{
    Orientation2, Orientation3, Point, Point2, Point3, Pose2, Pose3, Rotation2, Rotation3,
    Transform, Vector2, Vector3,
};

/// A rigid coordinate mapping from `From` to `To`.
///
/// Points are rotated, then translated; vectors are only rotated.
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

    /// Returns the inverse mapping, swapping source and destination frames.
    pub fn inverse(&self) -> Transform<To, From, nalgebra::Isometry<T, Rotation, DIMENSION>> {
        Transform::<To, From, _>::wrap(self.inner.inverse())
    }

    /// Returns the source frame's origin expressed in the destination frame.
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
    /// Creates an isometry from a translation in the destination frame and an angle in radians.
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

    /// Creates a pure rotation by the given angle in radians, with zero translation.
    pub fn from_angle(angle: T) -> Self {
        Self::wrap(nalgebra::Isometry2::rotation(angle))
    }

    /// Drops the local frame `From`, leaving a pose in the reference frame `To`.
    pub fn as_pose(&self) -> Pose2<To, T> {
        Pose2::wrap(self.inner)
    }

    /// Returns the rotation component as a transform from the source frame to the destination frame.
    pub fn rotation(&self) -> Rotation2<From, To, T> {
        Rotation2::wrap(self.inner.rotation)
    }

    /// Returns the rotation angle in radians.
    pub fn angle(&self) -> T {
        self.inner.rotation.angle()
    }

    /// Returns the source frame's orientation expressed in the destination frame.
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
    /// Creates an isometry from a translation and orientation in the destination frame.
    pub fn from_parts(translation: Vector3<To, T>, orientation: Orientation3<To, T>) -> Self {
        Self::wrap(nalgebra::Isometry3::from_parts(
            translation.inner.into(),
            orientation.inner,
        ))
    }

    /// Creates a pure rotation from a scaled axis in the destination frame.
    ///
    /// The vector points along the rotation axis and its norm is the angle in radians.
    pub fn from_axis_angle(axis_angle: Vector3<To, T>) -> Self {
        Self::wrap(nalgebra::Isometry3::rotation(axis_angle.inner))
    }

    /// Creates a pure translation from an offset expressed in the destination frame.
    pub fn from_translation(translation: Vector3<To, T>) -> Self {
        translation.into()
    }

    /// Drops the local frame `From`, leaving a pose in the reference frame `To`.
    pub fn as_pose(&self) -> Pose3<To, T> {
        Pose3::wrap(self.inner)
    }

    /// Returns the rotation component as a transform from the source frame to the destination frame.
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
    use std::f32::consts::FRAC_PI_2;

    use approx::assert_abs_diff_eq;

    use crate::{Isometry2, Isometry3, Orientation2, point, vector};

    #[derive(Debug)]
    struct Robot;
    #[derive(Debug)]
    struct Field;

    #[test]
    fn isometry2_constructors_rotate_then_translate() {
        let translation = vector![<Field>, 2.0, 3.0];
        let from_angle = Isometry2::<Robot, Field>::new(translation, FRAC_PI_2);
        let from_parts = Isometry2::from_parts(translation, Orientation2::new(FRAC_PI_2));
        let source = point![<Robot>, 1.0, 0.0];
        let expected = point![<Field>, 2.0, 4.0];

        assert_abs_diff_eq!(from_angle * source, expected, epsilon = 1.0e-6);
        assert_abs_diff_eq!(from_parts * source, expected, epsilon = 1.0e-6);
        assert_abs_diff_eq!(
            from_parts * vector![<Robot>, 1.0, 0.0],
            vector![<Field>, 0.0, 1.0],
            epsilon = 1.0e-6,
        );
    }

    #[test]
    fn isometry2_pure_rotation_preserves_the_angle_and_has_no_translation() {
        let isometry = Isometry2::<Robot, Field>::from_angle(FRAC_PI_2);

        assert_abs_diff_eq!(isometry.translation(), point![0.0, 0.0]);
        assert_abs_diff_eq!(isometry.angle(), FRAC_PI_2);
        assert_abs_diff_eq!(
            isometry.rotation() * vector![<Robot>, 1.0, 0.0],
            vector![<Field>, 0.0, 1.0],
            epsilon = 1.0e-6,
        );
    }

    #[test]
    fn isometry3_pure_rotation_uses_a_scaled_axis_and_has_no_translation() {
        let isometry = Isometry3::<Robot, Field>::from_axis_angle(vector![0.0, 0.0, FRAC_PI_2]);

        assert_abs_diff_eq!(isometry.translation(), point![0.0, 0.0, 0.0]);
        assert_abs_diff_eq!(
            isometry.rotation() * vector![<Robot>, 1.0, 0.0, 0.0],
            vector![<Field>, 0.0, 1.0, 0.0],
            epsilon = 1.0e-6,
        );
    }
}
