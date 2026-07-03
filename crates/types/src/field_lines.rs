use coordinate_systems::Pixel;
use geometry::line_segment::LineSegment;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ProjectedFieldLines {
    pub field_lines: Vec<LineSegment<Pixel>>,
}
