use coordinate_systems::Framed;
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

impl<Frame, Inner> LookAt<Framed<Frame, Inner>> for Framed<Frame, Inner>
where
    Inner: LookAt<Inner>,
{
    type Rotation = Framed<Frame, Inner::Rotation>;

    fn look_at(&self, target: &Self) -> Self::Rotation {
        Self::Rotation::new(self.inner.look_at(&target.inner))
    }
}

