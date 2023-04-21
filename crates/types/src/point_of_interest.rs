use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub enum PointOfInterest {
    #[default]
    Forward,
    FieldMark {
        absolute_position: Point2<f32>,
    },
    Ball,
    Obstacle {
        absolute_position: Point2<f32>,
    },
}
