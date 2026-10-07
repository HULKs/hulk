use serde::{Deserialize, Serialize};

use linear_algebra::Point2;

use coordinate_systems::Field;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum PointOfInterest {
    Forward,
    FieldMark { absolute_position: Point2<Field> },
    Ball,
    Obstacle { absolute_position: Point2<Field> },
}
