use std::ops::RangeInclusive;

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    Serialize,
    PartialEq,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum FieldColorFunction {
    #[default]
    GreenChromaticity,
    Hsv,
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct FieldColorParameters {
    pub luminance: RangeInclusive<u8>,
    pub green_luminance: RangeInclusive<u8>,
    pub red_chromaticity: RangeInclusive<f32>,
    pub green_chromaticity: RangeInclusive<f32>,
    pub blue_chromaticity: RangeInclusive<f32>,
    pub hue: RangeInclusive<u16>,
    pub saturation: RangeInclusive<u8>,
}

impl Default for FieldColorParameters {
    fn default() -> Self {
        Self {
            luminance: 0..=255,
            green_luminance: 0..=255,
            red_chromaticity: 0.0..=1.0,
            green_chromaticity: 0.0..=1.0,
            blue_chromaticity: 0.0..=1.0,
            hue: 0..=360,
            saturation: 0..=255,
        }
    }
}
