use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::color::Rgb;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct Leds {
    pub left_ear: Ear,
    pub right_ear: Ear,
    pub chest: Rgb,
    pub left_foot: Rgb,
    pub right_foot: Rgb,
    pub left_eye: Eye,
    pub right_eye: Eye,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
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

impl Eye {
    pub fn percentage(rgb_positive: Rgb, rgb_negative: Rgb, fraction: f32) -> Self {
        Self {
            color_at_0: if fraction > f32::EPSILON {
                rgb_positive
            } else {
                Rgb::BLACK
            },
            color_at_45: if fraction > 1. / 8. {
                rgb_positive
            } else {
                rgb_negative
            },
            color_at_90: if fraction > 2. / 8. {
                rgb_positive
            } else {
                rgb_negative
            },
            color_at_135: if fraction > 3. / 8. {
                rgb_positive
            } else {
                rgb_negative
            },
            color_at_180: if fraction > 4. / 8. {
                rgb_positive
            } else {
                rgb_negative
            },
            color_at_225: if fraction > 5. / 8. {
                rgb_positive
            } else {
                rgb_negative
            },
            color_at_270: if fraction > 6. / 8. {
                rgb_positive
            } else {
                rgb_negative
            },
            color_at_315: if fraction > 7. / 8. {
                rgb_positive
            } else {
                rgb_negative
            },
        }
    }

    pub fn invert(self) -> Self {
        Self {
            color_at_0: self.color_at_0.invert(),
            color_at_45: self.color_at_45.invert(),
            color_at_90: self.color_at_90.invert(),
            color_at_135: self.color_at_135.invert(),
            color_at_180: self.color_at_180.invert(),
            color_at_225: self.color_at_225.invert(),
            color_at_270: self.color_at_270.invert(),
            color_at_315: self.color_at_315.invert(),
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
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

    pub fn percentage_ears(intensity: f32, ear_fraction: f32) -> Self {
        Self {
            intensity_at_0: if ear_fraction > 0.0 { intensity } else { 0.0 },
            intensity_at_36: if ear_fraction > 0.1 { intensity } else { 0.0 },
            intensity_at_72: if ear_fraction > 0.2 { intensity } else { 0.0 },
            intensity_at_108: if ear_fraction > 0.3 { intensity } else { 0.0 },
            intensity_at_144: if ear_fraction > 0.4 { intensity } else { 0.0 },
            intensity_at_180: if ear_fraction > 0.5 { intensity } else { 0.0 },
            intensity_at_216: if ear_fraction > 0.6 { intensity } else { 0.0 },
            intensity_at_252: if ear_fraction > 0.7 { intensity } else { 0.0 },
            intensity_at_288: if ear_fraction > 0.8 { intensity } else { 0.0 },
            intensity_at_324: if ear_fraction > 0.9 { intensity } else { 0.0 },
        }
    }

    pub fn invert(self) -> Self {
        Self {
            intensity_at_0: (1.0 - self.intensity_at_0).abs(),
            intensity_at_36: (1.0 - self.intensity_at_36).abs(),
            intensity_at_72: (1.0 - self.intensity_at_72).abs(),
            intensity_at_108: (1.0 - self.intensity_at_108).abs(),
            intensity_at_144: (1.0 - self.intensity_at_144).abs(),
            intensity_at_180: (1.0 - self.intensity_at_180).abs(),
            intensity_at_216: (1.0 - self.intensity_at_216).abs(),
            intensity_at_252: (1.0 - self.intensity_at_252).abs(),
            intensity_at_288: (1.0 - self.intensity_at_288).abs(),
            intensity_at_324: (1.0 - self.intensity_at_324).abs(),
        }
    }
}
