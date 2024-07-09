use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::color::{Hsv, Intensity, Rgb, YCbCr444};

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

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct FieldColor {
    pub function: FieldColorFunction,
    pub luminance_threshold: f32,
    pub red_chromaticity_threshold: f32,
    pub blue_chromaticity_threshold: f32,
    pub green_chromaticity_threshold: f32,
    pub green_luminance_threshold: f32,
    pub hue_low_threshold: f32,
    pub hue_high_threshold: f32,
    pub saturation_low_threshold: f32,
    pub saturation_high_threshold: f32,
}

impl FieldColor {
    pub fn get_intensity(&self, color: YCbCr444) -> Intensity {
        let rgb = Rgb::from(color);

        match self.function {
            FieldColorFunction::GreenChromaticity => {
                let chromaticity = rgb.convert_to_rgchromaticity();
                if (chromaticity.red > self.red_chromaticity_threshold
                    || (1.0 - chromaticity.red - chromaticity.green)
                        > self.blue_chromaticity_threshold
                    || chromaticity.green < self.green_chromaticity_threshold
                    || (rgb.green as f32) < self.green_luminance_threshold)
                    && (rgb.get_luminance() as f32) > self.luminance_threshold
                {
                    Intensity::Low
                } else {
                    Intensity::High
                }
            }
            FieldColorFunction::Hsv => {
                let hsv = Hsv::from(rgb);
                let (h, s, v) = (hsv.h as f32, hsv.s as f32, hsv.v as f32);

                if v < self.luminance_threshold
                    || h < self.hue_low_threshold
                    || h > self.hue_high_threshold
                    || s < self.saturation_low_threshold
                    || s > self.saturation_high_threshold
                {
                    Intensity::Low
                } else {
                    Intensity::High
                }
            }
        }
    }
}
