use geometry::line::Line2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use coordinate_systems::Pixel;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct ProjectedFieldLines {
    pub top: Vec<Line2<Pixel>>,
    pub bottom: Vec<Line2<Pixel>>,
}
