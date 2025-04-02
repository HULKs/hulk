use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use linear_algebra::{vector, Vector2};

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
