use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

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
    #[default]
    UndefinedPose,
}

#[derive(Debug, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct PoseKindPosition<Frame> {
    pub pose_kind: PoseKind,
    pub position: Point2<Frame>,
}
