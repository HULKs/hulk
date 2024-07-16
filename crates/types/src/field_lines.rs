use coordinate_systems::Pixel;
use geometry::line_segment::LineSegment;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ProjectedFieldLines {
    pub top: Vec<LineSegment<Pixel>>,
    pub bottom: Vec<LineSegment<Pixel>>,
}
