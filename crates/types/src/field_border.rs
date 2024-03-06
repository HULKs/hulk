use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use linear_algebra::Point2;

use crate::{coordinate_systems::Pixel, line::Line2};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FieldBorder {
    pub border_lines: Vec<Line2<Pixel>>,
}

impl FieldBorder {
    pub fn is_inside_field(&self, point: Point2<Pixel>) -> bool {
        self.border_lines.iter().all(|line| line.is_above(point))
    }
}
