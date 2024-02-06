use coordinate_systems::Framed;
use nalgebra::Point2;
use serde::{Deserialize, Serialize};

use crate::coordinate_systems::Ground;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct KickTarget {
    pub position: Framed<Ground, Point2<f32>>,
    pub strength: Option<f32>,
}

impl KickTarget {
    pub fn new(position: Framed<Ground, Point2<f32>>) -> Self {
        Self {
            position,
            strength: None,
        }
    }

    pub fn new_with_strength(position: Framed<Ground, Point2<f32>>, strength: f32) -> Self {
        Self {
            position,
            strength: Some(strength),
        }
    }
}
