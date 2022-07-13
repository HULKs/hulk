use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SonarObstacle {
    pub position_in_robot: Point2<f32>,
}
