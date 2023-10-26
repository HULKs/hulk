use nalgebra::{Point2, UnitComplex, Vector2};

pub trait LookAt<Target> {
    type Rotation;
    fn look_at(&self, target: &Target) -> Self::Rotation;
}

impl LookAt<Point2<f32>> for Point2<f32> {
    type Rotation = UnitComplex<f32>;

    fn look_at(&self, target: &Point2<f32>) -> Self::Rotation {
        UnitComplex::rotation_between(&Vector2::x(), &(target - self))
    }
}
