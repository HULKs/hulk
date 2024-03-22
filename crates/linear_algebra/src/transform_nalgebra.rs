use nalgebra::{AbstractRotation, SimdRealField};

use crate::{
    Isometry, Isometry2, Isometry3, Orientation2, Orientation3, Point, Point2, Point3, Pose,
    Transform, UnitComplex, Vector2, Vector3,
};

// Isometry

impl<From, To, Scalar, const DIMENSION: usize, Rotation>
    Isometry<From, To, DIMENSION, Scalar, Rotation>
where
    Scalar::Element: SimdRealField,
    Scalar: SimdRealField,
    Rotation: AbstractRotation<Scalar, DIMENSION>,
{
    pub fn identity() -> Self {
        Self::wrap(nalgebra::Isometry::identity())
    }

    pub fn inverse(&self) -> Transform<To, From, nalgebra::Isometry<Scalar, Rotation, DIMENSION>> {
        Transform::<To, From, _>::wrap(self.inner.inverse())
    }

    pub fn translation(&self) -> Point<To, DIMENSION, Scalar> {
        Point::wrap(self.inner.translation.vector.clone().into())
    }
}

impl<From, To, Scalar> Transform<From, To, nalgebra::Isometry2<Scalar>>
where
    Scalar::Element: SimdRealField,
    Scalar: SimdRealField,
{
    pub fn new(translation: Vector2<To, Scalar>, angle: Scalar) -> Self {
        Transform::wrap(nalgebra::Isometry2::new(translation.inner, angle))
    }
}

impl<From, To, Scalar> Isometry3<From, To, Scalar>
where
    Scalar::Element: SimdRealField,
    Scalar: SimdRealField,
{
    pub fn from_parts(
        translation: Vector3<To, Scalar>,
        orientation: Orientation3<To, Scalar>,
    ) -> Self {
        Self::wrap(nalgebra::Isometry3::from_parts(
            translation.inner.into(),
            orientation.inner,
        ))
    }

    pub fn rotation(axisangle: Vector3<To, Scalar>) -> Self {
        Self::wrap(nalgebra::Isometry3::rotation(axisangle.inner))
    }
}

impl<From, To> Transform<From, To, nalgebra::Isometry2<f32>> {
    pub fn rotation(angle: f32) -> Self {
        Self::wrap(nalgebra::Isometry2::rotation(angle))
    }

    pub fn as_pose(&self) -> Pose<To> {
        Pose::wrap(self.inner)
    }

    pub fn orientation(&self) -> Orientation2<To> {
        Orientation2::wrap(self.inner.rotation)
    }
}

impl<From, To> core::convert::From<Vector2<To, f32>> for Isometry2<From, To, f32> {
    fn from(value: Vector2<To>) -> Self {
        Self::wrap(nalgebra::Isometry::from(value.inner))
    }
}

impl<From, To> core::convert::From<Vector3<To, f32>> for Isometry3<From, To, f32> {
    fn from(value: Vector3<To>) -> Self {
        Self::wrap(nalgebra::Isometry::from(value.inner))
    }
}

impl<From, To> core::convert::From<Point2<To, f32>> for Isometry2<From, To, f32> {
    fn from(value: Point2<To>) -> Self {
        Self::wrap(nalgebra::Isometry::from(value.inner))
    }
}

impl<From, To> core::convert::From<Point3<To, f32>> for Isometry3<From, To, f32> {
    fn from(value: Point3<To>) -> Self {
        Self::wrap(nalgebra::Isometry::from(value.inner))
    }
}

impl<From, To> core::convert::From<nalgebra::UnitQuaternion<f32>> for Isometry3<From, To, f32> {
    fn from(value: nalgebra::UnitQuaternion<f32>) -> Self {
        Self::wrap(nalgebra::Isometry::from_parts(
            nalgebra::Translation::identity(),
            value,
        ))
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

    pub fn as_orientation(&self) -> Orientation2<To> {
        Orientation2::wrap(self.inner)
    }

}

impl<Frame> UnitComplex<Frame, Frame> {
    pub fn rotation_between(a: Vector2<Frame>, b: Vector2<Frame>) -> Self {
        UnitComplex::wrap(nalgebra::UnitComplex::rotation_between(&a.inner, &b.inner))
    }
}
