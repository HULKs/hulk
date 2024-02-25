use serde::{Deserialize, Serialize};

use coordinate_systems::Point2;

use crate::coordinate_systems::Ground;

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
