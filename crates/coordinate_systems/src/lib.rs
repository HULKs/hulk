mod framed;
mod framed_nalgebra;
mod into;
mod transform;
mod transform_nalgebra;

pub use framed::Framed;
pub use framed_nalgebra::{center, distance, distance_squared};
pub use into::{IntoFramed, IntoTransform};
pub use transform::Transform;

#[macro_export]
macro_rules! transform {
    ($source:ty => $destination:ty; $inner:expr) => {
        $inner.framed_transform::<$source, $destination>()
    };
}

pub type Point<Frame, const DIMENSION: usize, Scalar = f32> =
    Framed<Frame, nalgebra::Point<Scalar, DIMENSION>>;
pub type Point2<Frame, Scalar = f32> = Point<Frame, 2, Scalar>;
pub type Point3<Frame, Scalar = f32> = Point<Frame, 3, Scalar>;

#[macro_export]
macro_rules! point {
    ($($parameters:expr),* $(,)?) => {
        coordinate_systems::Framed::wrap(nalgebra::point![$($parameters),*])

    };
}

pub type Vector<Frame, const DIMENSION: usize, Scalar = f32> =
    Framed<Frame, nalgebra::SVector<Scalar, DIMENSION>>;
pub type Vector2<Frame, Scalar = f32> = Vector<Frame, 2, Scalar>;
pub type Vector3<Frame, Scalar = f32> = Vector<Frame, 3, Scalar>;

#[macro_export]
macro_rules! vector {
    ($($parameters:expr),* $(,)?) => {
        coordinate_systems::Framed::wrap(nalgebra::vector![$($parameters),*])
    };
}

pub type Orientation2<Frame, Scalar = f32> = Framed<Frame, nalgebra::UnitComplex<Scalar>>;
pub type Orientation3<Frame, Scalar = f32> = Framed<Frame, nalgebra::UnitQuaternion<Scalar>>;
pub type UnitComplex<From, To, Scalar = f32> = Transform<From, To, nalgebra::UnitComplex<Scalar>>;

pub type Isometry<
    From,
    To,
    const DIMENSION: usize,
    Scalar = f32,
    Rotation = nalgebra::UnitComplex<f32>,
> = Transform<From, To, nalgebra::Isometry<Scalar, Rotation, DIMENSION>>;
pub type Isometry2<From, To, Scalar = f32> =
    Isometry<From, To, 2, Scalar, nalgebra::UnitComplex<Scalar>>;
pub type Isometry3<From, To, Scalar = f32> =
    Isometry<From, To, 3, Scalar, nalgebra::UnitQuaternion<Scalar>>;

pub type Pose<Frame, Scalar = f32> = Framed<Frame, nalgebra::Isometry2<Scalar>>;
