use serde::{Deserialize, Serialize};

use coordinate_systems::Ground;
use linear_algebra::Point2;
use serialize_hierarchy::SerializeHierarchy;

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FootBumperObstacle {
    pub position: Point2<Ground>,
}

impl From<Point2<Ground>> for FootBumperObstacle {
    fn from(position: Point2<Ground>) -> Self {
        Self { position }
    }
}
