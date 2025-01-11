use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::field_dimensions::GlobalFieldSide;

#[derive(
    Debug,
    Default,
    Clone,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    PartialEq,
    Eq,
)]
pub enum PoseKind {
    AboveHeadArms,
    FreeKickPose {
        global_field_side: GlobalFieldSide,
    },
    #[default]
    UndefinedPose,
}

#[derive(Debug, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct PoseKindPosition<Frame> {
    pub pose_kind: PoseKind,
    pub position: Point2<Frame>,
}
