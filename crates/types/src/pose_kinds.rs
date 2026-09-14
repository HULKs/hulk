use linear_algebra::Point2;
use serde::{Deserialize, Serialize};

use crate::field_dimensions::GlobalFieldSide;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PoseKind {
    Ready,
    FreeKick { global_field_side: GlobalFieldSide },
    UndefinedPose,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoseKindPosition<Frame> {
    pub pose_kind: PoseKind,
    pub position: Point2<Frame>,
}
