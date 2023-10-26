use nalgebra::{vector, Vector2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Orientation {
    Clockwise,
    Counterclockwise,
    Colinear,
}

impl Orientation {
    pub fn rotate_vector_90_degrees(&self, subject: Vector2<f32>) -> Vector2<f32> {
        match self {
            Orientation::Clockwise => vector![subject.y, -subject.x],
            Orientation::Counterclockwise => vector![-subject.y, subject.x],
            Orientation::Colinear => subject,
        }
    }
}
