use std::ops::Neg;

use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};

use linear_algebra::{vector, Vector2};
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
    pub fn rotate_vector_90_degrees<Frame>(&self, subject: Vector2<Frame>) -> Vector2<Frame> {
        match self {
            Direction::Clockwise => vector![subject.y(), -subject.x()],
            Direction::Counterclockwise => vector![-subject.y(), subject.x()],
            Direction::Colinear => subject,
        }
    }

    pub fn angle_sign<T: One + Zero + Neg<Output = T>>(self) -> T {
        match self {
            Direction::Clockwise => -T::one(),
            Direction::Counterclockwise => T::one(),
            Direction::Colinear => T::zero(),
        }
    }
}
