use coordinate_systems::{Framed, IntoFramed};
use nalgebra::{vector, Vector2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Orientation {
    Clockwise,
    Counterclockwise,
    Colinear,
}

impl Orientation {
    pub fn rotate_vector_90_degrees<Frame>(
        &self,
        subject: Framed<Frame, Vector2<f32>>,
    ) -> Framed<Frame, Vector2<f32>> {
        match self {
            Orientation::Clockwise => vector![subject.y(), -subject.x()].framed(),
            Orientation::Counterclockwise => vector![-subject.y(), subject.x()].framed(),
            Orientation::Colinear => subject,
        }
    }
}
