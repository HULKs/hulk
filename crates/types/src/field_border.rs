use geometry::line::Line2;
use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use linear_algebra::Point2;
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FieldBorder {
    pub border_lines: Vec<Line2<Pixel>>,
}

impl FieldBorder {
    pub fn is_inside_field(&self, point: Point2<Pixel>) -> bool {
        self.border_lines.iter().all(|line| line.is_above(point))
    }
}
