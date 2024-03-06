use geometry::{circle::Circle, rectangle::Rectangle};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use coordinate_systems::Field;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub enum RuleObstacle {
    Circle(Circle<Field>),
    Rectangle(Rectangle<Field>),
}
