use serde::{Deserialize, Serialize};

use coordinate_systems::Ground;
use linear_algebra::Orientation2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{motion_command::OrientationMode, planned_path::PathSegment};

#[derive(Clone, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct DribblePathPlan {
    pub orientation_mode: OrientationMode,
    pub target_orientation: Orientation2<Ground>,
    pub path: Vec<PathSegment>,
}
