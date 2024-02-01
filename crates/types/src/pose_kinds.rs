use linear_algebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Debug, Clone, Serialize, Deserialize, SerializeHierarchy, PartialEq, Eq)]
pub enum PoseKind {
    AboveHeadArms,
    ArmsBySide,
}

#[derive(Debug, Clone, Serialize, Deserialize, SerializeHierarchy)]
pub struct PoseKindPosition<Frame> {
    pub pose_kind: PoseKind,
    pub position: Point2<Frame>,
}
