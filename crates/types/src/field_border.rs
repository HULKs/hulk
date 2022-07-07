use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::Line2;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FieldBorder {
    pub border_lines: Vec<Line2>,
}

impl FieldBorder {
    pub fn is_inside_field(&self, point: Point2<f32>) -> bool {
        self.border_lines.iter().all(|line| line.is_above(point))
    }
}
