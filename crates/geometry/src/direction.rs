use std::{f32::consts::TAU, ops::Neg};

use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};

use linear_algebra::{vector, Orientation2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum Direction {
    Clockwise,
    Counterclockwise,
    Collinear,
}

impl Direction {
    pub fn angle_sign<T: One + Zero + Neg<Output = T>>(self) -> T {
        match self {
            Direction::Clockwise => -T::one(),
            Direction::Counterclockwise => T::one(),
            Direction::Collinear => T::zero(),
        }
    }
}

pub trait Rotate90Degrees {
    fn rotate_90_degrees(&self, direction: Direction) -> Self;
}

impl<Frame> Rotate90Degrees for Vector2<Frame> {
    fn rotate_90_degrees(&self, direction: Direction) -> Self {
        match direction {
            Direction::Clockwise => vector![self.y(), -self.x()],
            Direction::Counterclockwise => vector![-self.y(), self.x()],
            Direction::Collinear => *self,
        }
    }
}

pub trait AngleTo {
    fn angle_to(&self, other: Self, direction: Direction) -> f32;
}

impl<Frame> AngleTo for Orientation2<Frame> {
    fn angle_to(&self, other: Self, direction: Direction) -> f32 {
        (self.rotation_to(other).angle() * direction.angle_sign::<f32>()).rem_euclid(TAU)
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    use approx::assert_abs_diff_eq;
    use linear_algebra::Orientation2;

    use crate::direction::{AngleTo, Direction};

    struct SomeFrame;

    #[test]
    fn angle_to() {
        let angle_0deg = Orientation2::<SomeFrame>::new(0.0);
        let angle_45deg = Orientation2::<SomeFrame>::new(FRAC_PI_4);
        let angle_90deg = Orientation2::<SomeFrame>::new(FRAC_PI_2);
        let angle_180deg = Orientation2::<SomeFrame>::new(PI);
        let angle_315deg = Orientation2::<SomeFrame>::new(-FRAC_PI_4);

        assert_abs_diff_eq!(
            angle_0deg.angle_to(angle_45deg, Direction::Clockwise),
            FRAC_PI_4 * 7.0
        );
        assert_abs_diff_eq!(
            angle_0deg.angle_to(angle_45deg, Direction::Counterclockwise),
            FRAC_PI_4
        );

        assert_abs_diff_eq!(
            angle_45deg.angle_to(angle_315deg, Direction::Clockwise),
            FRAC_PI_2
        );
        assert_abs_diff_eq!(
            angle_45deg.angle_to(angle_315deg, Direction::Counterclockwise),
            FRAC_PI_2 * 3.0
        );

        assert_abs_diff_eq!(
            angle_90deg.angle_to(angle_180deg, Direction::Clockwise),
            FRAC_PI_2 * 3.0
        );
        assert_abs_diff_eq!(
            angle_90deg.angle_to(angle_180deg, Direction::Counterclockwise),
            FRAC_PI_2
        );
    }
}
