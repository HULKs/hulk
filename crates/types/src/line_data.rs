use std::collections::HashSet;

use coordinate_systems::{Ground, Pixel};
use geometry::line::Line2;
use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Default, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct LineData {
    pub lines: Vec<Line2<Ground>>,
    pub used_segments: HashSet<Point2<Pixel, u16>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub enum LineDiscardReason {
    TooFewPoints,
    LineTooShort,
    LineTooLong,
    TooFarAway,
}
