use geometry::{circle::Circle, rectangle::Rectangle};
use linear_algebra::Point2;
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

impl RuleObstacle {
    pub fn contains(&self, point: Point2<Field>) -> bool {
        match self {
            RuleObstacle::Circle(circle) => circle.contains(point),
            RuleObstacle::Rectangle(rectangle) => rectangle.contains(point),
        }
    }
}
