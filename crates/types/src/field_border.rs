use geometry::{line::Line, line_segment::LineSegment};
use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct FieldBorder {
    pub border_lines: Vec<LineSegment<Pixel>>,
}

impl FieldBorder {
    pub fn is_inside_field(&self, point: Point2<Pixel>) -> bool {
        self.border_lines
            .iter()
            .all(|line_segment| Line::from(*line_segment).is_above(point))
    }
}
