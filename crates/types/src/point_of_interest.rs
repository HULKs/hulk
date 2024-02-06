use coordinate_systems::Framed;
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::coordinate_systems::Field;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub enum PointOfInterest {
    #[default]
    Forward,
    FieldMark {
        absolute_position: Framed<Field, Point2<f32>>,
    },
    Ball,
    Obstacle {
        absolute_position: Framed<Field, Point2<f32>>,
    },
}
