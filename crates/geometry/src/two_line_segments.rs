use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::line_segment::LineSegment;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PartialEq,
    PathSerialize,
    PathIntrospect,
    PathDeserialize,
)]
pub struct TwoLineSegments<Frame>(pub LineSegment<Frame>, pub LineSegment<Frame>);
