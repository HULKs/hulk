use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::motion_command::{Facing, FallDirection};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub enum FallState {
    Upright,
    Falling { direction: FallDirection },
    Fallen { facing: Facing },
}

impl Default for FallState {
    fn default() -> Self {
        Self::Upright
    }
}
