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
    Colinear,
}

impl Direction {
    pub fn angle_sign<T: One + Zero + Neg<Output = T>>(self) -> T {
        match self {
            Direction::Clockwise => -T::one(),
            Direction::Counterclockwise => T::one(),
            Direction::Colinear => T::zero(),
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
            Direction::Colinear => *self,
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
