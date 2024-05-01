use coordinate_systems::Pixel;
use geometry::line::Line2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ProjectedFieldLines {
    pub top: Vec<Line2<Pixel>>,
    pub bottom: Vec<Line2<Pixel>>,
}
