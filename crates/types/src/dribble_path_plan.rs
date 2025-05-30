use serde::{Deserialize, Serialize};

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{motion_command::OrientationMode, planned_path::PathSegment};

#[derive(Clone, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct DribblePathPlan {
    pub orientation_mode: OrientationMode,
    pub path: Vec<PathSegment>,
}
