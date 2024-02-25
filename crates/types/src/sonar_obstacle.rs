use serde::{Deserialize, Serialize};

use coordinate_systems::Point2;
use serialize_hierarchy::SerializeHierarchy;

use crate::coordinate_systems::Ground;

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SonarObstacle {
    pub position: Point2<Ground>,
}
