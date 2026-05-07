use std::collections::HashSet;

use coordinate_systems::{Ground, Pixel};
use geometry::line_segment::LineSegment;
use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Default,
    Debug,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    Message,
)]
pub struct LineData {
    pub lines: Vec<LineSegment<Ground>>,
    pub used_segments: HashSet<Point2<Pixel, u16>>,
}

#[derive(
    Clone, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect, Message,
)]
pub enum LineDiscardReason {
    TooFewPoints,
    LineTooShort,
    LineTooLong,
    TooFarAway,
}
