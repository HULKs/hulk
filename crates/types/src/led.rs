use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::Rgb;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct Leds {
    pub left_ear: Ear,
    pub right_ear: Ear,
    pub chest: Rgb,
    pub left_foot: Rgb,
    pub right_foot: Rgb,
    pub left_eye: Eye,
    pub right_eye: Eye,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct Eye {
    pub color_at_0: Rgb,
    pub color_at_45: Rgb,
    pub color_at_90: Rgb,
    pub color_at_135: Rgb,
    pub color_at_180: Rgb,
    pub color_at_225: Rgb,
    pub color_at_270: Rgb,
    pub color_at_315: Rgb,
}

impl From<Rgb> for Eye {
    fn from(rgb: Rgb) -> Self {
        Self {
            color_at_0: rgb,
            color_at_45: rgb,
            color_at_90: rgb,
            color_at_135: rgb,
            color_at_180: rgb,
            color_at_225: rgb,
            color_at_270: rgb,
            color_at_315: rgb,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct Ear {
    pub intensity_at_0: f32,
    pub intensity_at_36: f32,
    pub intensity_at_72: f32,
    pub intensity_at_108: f32,
    pub intensity_at_144: f32,
    pub intensity_at_180: f32,
    pub intensity_at_216: f32,
    pub intensity_at_252: f32,
    pub intensity_at_288: f32,
    pub intensity_at_324: f32,
}

impl Ear {
    pub fn full_ears(intensity: f32) -> Self {
        Self {
            intensity_at_0: intensity,
            intensity_at_36: intensity,
            intensity_at_72: intensity,
            intensity_at_108: intensity,
            intensity_at_144: intensity,
            intensity_at_180: intensity,
            intensity_at_216: intensity,
            intensity_at_252: intensity,
            intensity_at_288: intensity,
            intensity_at_324: intensity,
        }
    }

    pub fn percentage_ears(intensity: f32, percentage: i32) -> Self {
        Self {
            intensity_at_0: if percentage > 0 { intensity } else { 0.0 },
            intensity_at_36: if percentage > 10 { intensity } else { 0.0 },
            intensity_at_72: if percentage > 20 { intensity } else { 0.0 },
            intensity_at_108: if percentage > 30 { intensity } else { 0.0 },
            intensity_at_144: if percentage > 40 { intensity } else { 0.0 },
            intensity_at_180: if percentage > 50 { intensity } else { 0.0 },
            intensity_at_216: if percentage > 60 { intensity } else { 0.0 },
            intensity_at_252: if percentage > 70 { intensity } else { 0.0 },
            intensity_at_288: if percentage > 80 { intensity } else { 0.0 },
            intensity_at_324: if percentage > 90 { intensity } else { 0.0 },
        }
    }
}
