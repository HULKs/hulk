use nalgebra::{AbstractRotation, Isometry, Isometry2, SimdRealField};

use crate::{Isometry3, Orientation, Point, Pose, Transform, UnitComplex, Vector2};

// Isometry

impl<From, To, Scalar, Rotation, const DIMENSION: usize>
    Transform<From, To, Isometry<Scalar, Rotation, DIMENSION>>
where
    Scalar::Element: SimdRealField,
    Scalar: SimdRealField,
    Rotation: AbstractRotation<Scalar, DIMENSION>,
{
    pub fn identity() -> Self {
        Self::wrap(nalgebra::Isometry::identity())
    }

    pub fn inverse(&self) -> Transform<To, From, Isometry<Scalar, Rotation, DIMENSION>> {
        Transform::<To, From, _>::wrap(self.inner.inverse())
    }

    // TODO: naming kloppen
    pub fn origin(&self) -> Point<To, DIMENSION, Scalar> {
        Point::wrap(self.inner.translation.vector.clone().into())
    }
}

impl<From, To, Scalar> Transform<From, To, nalgebra::Isometry2<Scalar>>
where
    Scalar::Element: SimdRealField,
    Scalar: SimdRealField,
{
    pub fn new(translation: Vector2<To, Scalar>, angle: Scalar) -> Self {
        Transform::wrap(Isometry2::new(translation.inner, angle))
    }
}

impl<From, To, Scalar> Isometry3<From, To, Scalar>
where
    Scalar::Element: SimdRealField,
    Scalar: SimdRealField,
{
    pub fn translation(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Transform::wrap(nalgebra::Isometry3::translation(x, y, z))
    }
}

impl<From, To> Transform<From, To, nalgebra::Isometry2<f32>> {
    pub fn as_pose(&self) -> Pose<To> {
        Pose::wrap(self.inner)
    }

    pub fn orientation(&self) -> Orientation<To> {
        Orientation::wrap(self.inner.rotation)
    }
}

// UnitComplex

impl<From, To> UnitComplex<From, To> {
    pub fn new(angle: f32) -> Self {
        UnitComplex::wrap(nalgebra::UnitComplex::new(angle))
    }

    pub fn from_vector(direction: Vector2<To>) -> Self {
        UnitComplex::wrap(nalgebra::UnitComplex::rotation_between(
            &nalgebra::Vector2::x_axis(),
            &direction.inner,
        ))
    }

    pub fn angle(&self) -> f32 {
        self.inner.angle()
    }

    pub fn inverse(&self) -> UnitComplex<To, From> {
        Transform::<To, From, _>::wrap(self.inner.inverse())
    }
}

impl<Frame> UnitComplex<Frame, Frame> {
    pub fn rotation_between(a: Vector2<Frame>, b: Vector2<Frame>) -> Self {
        UnitComplex::wrap(nalgebra::UnitComplex::rotation_between(&a.inner, &b.inner))
    }
}
