use serde::{Deserialize, Serialize};

use linear_algebra::Point2;
use serialize_hierarchy::SerializeHierarchy;

use coordinate_systems::Ground;

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct SonarObstacle {
    pub position: Point2<Ground>,
}
