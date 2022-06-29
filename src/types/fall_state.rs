use serde::{Deserialize, Serialize};

use super::{Facing, FallDirection};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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
