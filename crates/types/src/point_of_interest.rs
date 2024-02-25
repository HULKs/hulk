use serde::{Deserialize, Serialize};

use coordinate_systems::Point2;
use serialize_hierarchy::SerializeHierarchy;

use crate::coordinate_systems::Field;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
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
