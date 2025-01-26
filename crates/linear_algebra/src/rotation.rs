use nalgebra::{RealField, SimdRealField};

use crate::{Orientation2, Orientation3, Transform, Vector2, Vector3};

pub type Rotation2<From, To, T = f32> = Transform<From, To, nalgebra::UnitComplex<T>>;
pub type Rotation3<Frame, To, T = f32> = Transform<Frame, To, nalgebra::UnitQuaternion<T>>;

impl<From, To, T> Rotation2<From, To, T>
where
    T: SimdRealField,
    T::Element: SimdRealField,
{
    pub fn new(angle: T) -> Self {
        Self::wrap(nalgebra::UnitComplex::new(angle))
    }

    pub fn from_vector(direction: Vector2<To, T>) -> Self
    where
        T: RealField,
    {
        Self::wrap(nalgebra::UnitComplex::rotation_between(
            &nalgebra::Vector2::x_axis(),
            &direction.inner,
        ))
    }

    pub fn angle(&self) -> T {
        self.inner.angle()
    }

    pub fn inverse(&self) -> Rotation2<To, From, T> {
        Rotation2::wrap(self.inner.inverse())
    }

    pub fn as_orientation(&self) -> Orientation2<To, T> {
        Orientation2::wrap(self.inner.clone())
    }
}

impl<From, To> Rotation2<From, To, f32> {
    pub fn clamp_angle<To2>(&self, min: f32, max: f32) -> Rotation2<From, To2, f32> {
        Rotation2::new(self.angle().clamp(min, max))
    }
}

impl<Frame, T> Rotation2<Frame, Frame, T>
where
    T: RealField,
{
    pub fn rotation_between(a: Vector2<Frame, T>, b: Vector2<Frame, T>) -> Self {
        Self::wrap(nalgebra::UnitComplex::rotation_between(&a.inner, &b.inner))
    }
}

impl<From, To, T> Rotation3<From, To, T>
where
    T: SimdRealField + Copy,
    T::Element: SimdRealField,
{
    pub fn new(axis_angle: Vector3<To, T>) -> Self {
        Self::wrap(nalgebra::UnitQuaternion::new(axis_angle.inner))
    }

    pub fn from_euler_angles(x: T, y: T, z: T) -> Self {
        Self::wrap(nalgebra::UnitQuaternion::from_euler_angles(x, y, z))
    }

    pub fn inverse(&self) -> Rotation3<To, From, T> {
        Transform::<To, From, _>::wrap(self.inner.inverse())
    }

    pub fn as_orientation(&self) -> Orientation3<To, T> {
        Orientation3::wrap(self.inner)
    }
}

impl<Frame, T> Rotation3<Frame, Frame, T>
where
    T: RealField,
{
    pub fn rotation_between(a: Vector3<Frame, T>, b: Vector3<Frame, T>) -> Option<Self> {
        Some(Self::wrap(nalgebra::UnitQuaternion::rotation_between(
            &a.inner, &b.inner,
        )?))
    }
}
