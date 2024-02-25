use serde::{Deserialize, Serialize};

use coordinate_systems::{vector, Vector2};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
}
