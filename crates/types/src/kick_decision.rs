use nalgebra::Isometry2;
use serde::{Deserialize, Serialize};

use crate::{KickVariant, Side};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KickDecision {
    pub variant: KickVariant,
    pub kicking_side: Side,
    pub relative_kick_pose: Isometry2<f32>,
    pub is_reached: bool,
}
