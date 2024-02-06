use coordinate_systems::Framed;
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{coordinate_systems::Pixel, line::Line2};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FieldBorder {
    pub border_lines: Vec<Line2<Pixel>>,
}

impl FieldBorder {
    pub fn is_inside_field(&self, point: Framed<Pixel, Point2<f32>>) -> bool {
        self.border_lines.iter().all(|line| line.is_above(point))
    }
}
