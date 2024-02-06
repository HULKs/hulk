use coordinate_systems::Framed;
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::coordinate_systems::Ground;

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SonarObstacle {
    pub position: Framed<Ground, Point2<f32>>,
}
