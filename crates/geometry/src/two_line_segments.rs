use serde::{Deserialize, Serialize};

use crate::line_segment::LineSegment;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct TwoLineSegments<Frame>(pub LineSegment<Frame>, pub LineSegment<Frame>);
