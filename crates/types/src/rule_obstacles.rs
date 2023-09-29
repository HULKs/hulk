use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::geometry::{Circle, Rectangle};
#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub enum RuleObstacle {
    Circle(Circle),
    Rectangle(Rectangle),
}
