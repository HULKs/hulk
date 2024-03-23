use nalgebra::{ArrayStorage, Const, Matrix, SVector, Scalar};
use num_traits::{identities::Zero, One};

use crate::{
    Framed, Isometry2, Orientation2, Orientation3, Point, Point2, Point3, Pose, UnitComplex,
    Vector, Vector2, Vector3,
};

// Vectors

impl<Frame, const DIMENSION: usize> Framed<Frame, SVector<f32, DIMENSION>> {
    pub fn zeros() -> Self {
        Self::wrap(SVector::zeros())
    }

    pub fn as_point(&self) -> Point<Frame, DIMENSION> {
        Framed::wrap(nalgebra::Point::from(self.inner))
    }

    pub fn normalize(&self) -> Self {
        Self::wrap(self.inner.normalize())
    }

    pub fn try_normalize(&self, min_norm: f32) -> Option<Self> {
        Some(Self::wrap(self.inner.try_normalize(min_norm)?))
    }

    pub fn cap_magnitude(&self, max: f32) -> Self {
        Self::wrap(self.inner.cap_magnitude(max))
    }

    pub fn unscale(&self, real: f32) -> Self {
        Self::wrap(self.inner.unscale(real))
    }

    pub fn transpose(
        &self,
    ) -> Framed<Frame, Matrix<f32, Const<1>, Const<DIMENSION>, ArrayStorage<f32, 1, DIMENSION>>>
    {
        Framed::wrap(self.inner.transpose())
    }

    pub fn norm(&self) -> f32 {
        self.inner.norm()
    }

    pub fn norm_squared(&self) -> f32 {
        self.inner.norm_squared()
    }

    pub fn dot(&self, rhs: Self) -> f32 {
        self.inner.dot(&rhs.inner)
    }

    pub fn angle(&self, rhs: Self) -> f32 {
        self.inner.angle(&rhs.inner)
    }

    pub fn component_mul(&self, rhs: Self) -> Self {
        Self::wrap(self.inner.component_mul(&rhs.inner))
    }
}

impl<Frame, From, const DIMENSION: usize> Framed<Frame, SVector<From, DIMENSION>>
where
    From: Scalar,
{
    pub fn map<F, To>(&self, f: F) -> Framed<Frame, SVector<To, DIMENSION>>
    where
        To: Scalar,
        F: FnMut(From) -> To,
    {
        Framed::wrap(self.inner.map(f))
    }

    pub fn zip_map<From2, To, F>(
        &self,
        rhs: Framed<Frame, SVector<From2, DIMENSION>>,
        f: F,
    ) -> Framed<Frame, SVector<To, DIMENSION>>
    where
        From2: Scalar,
        To: Scalar,
        F: FnMut(From, From2) -> To,
    {
        Framed::wrap(self.inner.zip_map(&rhs.inner, f))
    }
}

impl<Frame, Scalar> Framed<Frame, SVector<Scalar, 2>>
where
    Scalar: nalgebra::Scalar + Zero + One + Copy,
{
    pub fn x(&self) -> Scalar {
        self.inner.x
    }

    pub fn y(&self) -> Scalar {
        self.inner.y
    }

    pub fn x_axis() -> Self {
        Self::wrap(*SVector::x_axis())
    }

    pub fn y_axis() -> Self {
        Self::wrap(*SVector::y_axis())
    }
}

impl<Frame, Scalar> Framed<Frame, SVector<Scalar, 3>>
where
    Scalar: nalgebra::Scalar + Zero + One + Copy,
{
    pub fn x(&self) -> Scalar {
        self.inner.x
    }

    pub fn y(&self) -> Scalar {
        self.inner.y
    }

    pub fn z(&self) -> Scalar {
        self.inner.z
    }

    pub fn x_axis() -> Self {
        Self::wrap(*SVector::x_axis())
    }

    pub fn y_axis() -> Self {
        Self::wrap(*SVector::y_axis())
    }

    pub fn z_axis() -> Self {
        Self::wrap(*SVector::z_axis())
    }

    pub fn xy(&self) -> Vector2<Frame, Scalar> {
        Vector2::wrap(self.inner.xy())
    }

    pub fn xz(&self) -> Vector2<Frame, Scalar> {
        Vector2::wrap(self.inner.xz())
    }

    pub fn yz(&self) -> Vector2<Frame, Scalar> {
        Vector2::wrap(self.inner.yz())
    }
}

impl<Frame, const DIMENSION: usize, Scalar> From<nalgebra::Point<Scalar, DIMENSION>>
    for Vector<Frame, DIMENSION, Scalar>
where
    Scalar: nalgebra::Scalar,
{
    fn from(value: nalgebra::Point<Scalar, DIMENSION>) -> Self {
        Self::wrap(value.coords)
    }
}

// Points

pub fn distance<Frame, const DIMENSION: usize>(
    p1: Point<Frame, DIMENSION>,
    p2: Point<Frame, DIMENSION>,
) -> f32 {
    nalgebra::distance(&p1.inner, &p2.inner)
}

pub fn distance_squared<Frame, const DIMENSION: usize>(
    p1: Point<Frame, DIMENSION>,
    p2: Point<Frame, DIMENSION>,
) -> f32 {
    nalgebra::distance_squared(&p1.inner, &p2.inner)
}

pub fn center<Frame, const DIMENSION: usize>(
    p1: Point<Frame, DIMENSION>,
    p2: Point<Frame, DIMENSION>,
) -> Point<Frame, DIMENSION> {
    Framed::wrap(nalgebra::center(&p1.inner, &p2.inner))
}

impl<Frame, const DIMENSION: usize> Point<Frame, DIMENSION> {
    pub fn origin() -> Self {
        Self::wrap(nalgebra::Point::origin())
    }

    pub fn coords(&self) -> Framed<Frame, SVector<f32, DIMENSION>> {
        Framed::wrap(self.inner.coords)
    }
}

impl<Frame, From, const DIMENSION: usize> Point<Frame, DIMENSION, From>
where
    From: Scalar,
{
    pub fn map<F, To>(&self, f: F) -> Point<Frame, DIMENSION, To>
    where
        To: Scalar,
        F: FnMut(From) -> To,
    {
        Framed::wrap(self.inner.map(f))
    }
}

impl<Frame, Scalar> Point2<Frame, Scalar>
where
    Scalar: nalgebra::Scalar + Zero + One + Copy,
{
    pub fn x(&self) -> Scalar {
        self.inner.x
    }

    pub fn y(&self) -> Scalar {
        self.inner.y
    }
}

impl<Frame, Scalar> Point3<Frame, Scalar>
where
    Scalar: nalgebra::Scalar + Zero + One + Copy,
{
    pub fn x(&self) -> Scalar {
        self.inner.x
    }

    pub fn y(&self) -> Scalar {
        self.inner.y
    }

    pub fn z(&self) -> Scalar {
        self.inner.z
    }

    pub fn xy(&self) -> Point2<Frame, Scalar> {
        Point2::wrap(self.inner.xy())
    }

    pub fn xz(&self) -> Point2<Frame, Scalar> {
        Point2::wrap(self.inner.xz())
    }

    pub fn yz(&self) -> Point2<Frame, Scalar> {
        Point2::wrap(self.inner.yz())
    }
}

impl<Frame, const DIMENSION: usize, Scalar> From<nalgebra::SVector<Scalar, DIMENSION>>
    for Point<Frame, DIMENSION, Scalar>
where
    Scalar: nalgebra::Scalar,
{
    fn from(value: nalgebra::SVector<Scalar, DIMENSION>) -> Self {
        Self::wrap(value.into())
    }
}

// Pose

impl<Frame> Pose<Frame> {
    pub fn new(translation: Vector2<Frame, f32>, angle: f32) -> Self {
        Self::wrap(nalgebra::Isometry2::new(translation.inner, angle))
    }

    pub fn from_parts(position: Point2<Frame, f32>, orientation: Orientation2<Frame>) -> Self {
        Self::wrap(nalgebra::Isometry2::from_parts(
            position.inner.into(),
            orientation.inner,
        ))
    }

    pub fn as_transform<From>(&self) -> Isometry2<From, Frame> {
        Isometry2::wrap(self.inner)
    }

    pub fn position(&self) -> Point2<Frame> {
        Point2::wrap(self.inner.translation.vector.into())
    }

    pub fn orientation(&self) -> Orientation2<Frame> {
        Orientation2::wrap(self.inner.rotation)
    }

    pub fn angle(&self) -> f32 {
        self.inner.rotation.angle()
    }
}

impl<Frame> From<Point2<Frame>> for Pose<Frame> {
    fn from(value: Point2<Frame>) -> Self {
        Self::wrap(nalgebra::Isometry2::from(value.inner))
    }
}

// Orientation

impl<Frame> Orientation2<Frame> {
    pub fn new(angle: f32) -> Self {
        Self::wrap(nalgebra::UnitComplex::new(angle))
    }

    pub fn identity() -> Self {
        Self::wrap(nalgebra::UnitComplex::identity())
    }

    pub fn inverse(&self) -> Self {
        Self::wrap(self.inner.inverse())
    }

    pub fn from_cos_sin_unchecked(cos: f32, sin: f32) -> Self {
        Self::wrap(nalgebra::UnitComplex::from_cos_sin_unchecked(cos, sin))
    }

    pub fn from_vector(direction: Vector2<Frame>) -> Self {
        Self::wrap(nalgebra::UnitComplex::rotation_between(
            &nalgebra::Vector2::x_axis(),
            &direction.inner,
        ))
    }

    pub fn as_transform<From>(&self) -> UnitComplex<From, Frame> {
        UnitComplex::wrap(self.inner)
    }

    pub fn angle(&self) -> f32 {
        self.inner.angle()
    }

    pub fn slerp(&self, other: Self, t: f32) -> Self {
        Self::wrap(self.inner.slerp(&other.inner, t))
    }

    pub fn rotation_to(&self, other: Self) -> UnitComplex<Frame, Frame> {
        UnitComplex::wrap(self.inner.rotation_to(&other.inner))
    }
}

impl<Frame> Orientation3<Frame> {
    pub fn new(axis_angle: Vector3<Frame>) -> Self {
        Self::wrap(nalgebra::UnitQuaternion::new(axis_angle.inner))
    }

    pub fn from_euler_angles(roll: f32, pitch: f32, yaw: f32) -> Self {
        Self::wrap(nalgebra::UnitQuaternion::from_euler_angles(
            roll, pitch, yaw,
        ))
    }

    pub fn inverse(&self) -> Self {
        Self::wrap(self.inner.inverse())
    }
}
