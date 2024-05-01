use serde::{Deserialize, Serialize};

use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use coordinate_systems::Field;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum PointOfInterest {
    #[default]
    Forward,
    FieldMark {
        absolute_position: Point2<Field>,
    },
    Ball,
    Obstacle {
        absolute_position: Point2<Field>,
    },
}
