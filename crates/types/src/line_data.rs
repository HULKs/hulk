use std::collections::HashSet;

use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::Line2;

#[derive(Clone, Default, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct LineData {
    pub lines_in_robot: Vec<Line2>,
    pub used_vertical_filtered_segments: HashSet<Point2<u16>>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct ImageLines {
    pub lines: Vec<Line2>,
    pub points: Vec<Point2<f32>>,
}
