use geometry::{circle::Circle, rectangle::Rectangle};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub enum RuleObstacle {
    Circle(Circle),
    Rectangle(Rectangle),
}
