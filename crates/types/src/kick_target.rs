use serde::{Deserialize, Serialize};

use linear_algebra::Point2;

use coordinate_systems::Ground;

use crate::motion_command::KickVariant;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct KickTarget {
    pub position: Point2<Ground>,
    pub strength: Option<f32>,
}

impl KickTarget {
    pub fn new(position: Point2<Ground>) -> Self {
        Self {
            position,
            strength: None,
        }
    }

    pub fn new_with_strength(position: Point2<Ground>, strength: f32) -> Self {
        Self {
            position,
            strength: Some(strength),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KickTargetWithKickVariants {
    pub kick_target: KickTarget,
    pub kick_variants: Vec<KickVariant>,
}
