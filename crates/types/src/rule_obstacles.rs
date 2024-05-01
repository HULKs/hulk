use geometry::{circle::Circle, rectangle::Rectangle};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use coordinate_systems::Field;

#[derive(
    Clone, Copy, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub enum RuleObstacle {
    Circle(Circle<Field>),
    Rectangle(Rectangle<Field>),
}
