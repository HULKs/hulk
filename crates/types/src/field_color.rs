use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::interpolated::Interpolated;

use super::{Intensity, Rgb, RgbChannel, YCbCr444};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FieldColor {
    pub red_chromaticity_threshold: Interpolated,
    pub blue_chromaticity_threshold: Interpolated,
    pub lower_green_chromaticity_threshold: Interpolated,
    pub upper_green_chromaticity_threshold: Interpolated,
    pub green_luminance_threshold: Interpolated,
}

impl FieldColor {
    pub fn get_intensity(&self, color: YCbCr444, interpolation_argument: Point2<f32>) -> Intensity {
        let rgb = Rgb::from(color);
        let red_chromaticity = rgb.get_chromaticity(RgbChannel::Red);
        let green_chromaticity = rgb.get_chromaticity(RgbChannel::Green);
        let blue_chromaticity = rgb.get_chromaticity(RgbChannel::Blue);
        if red_chromaticity
            > self
                .red_chromaticity_threshold
                .evaluate_at(interpolation_argument)
            || blue_chromaticity
                > self
                    .blue_chromaticity_threshold
                    .evaluate_at(interpolation_argument)
            || green_chromaticity
                < self
                    .lower_green_chromaticity_threshold
                    .evaluate_at(interpolation_argument)
            || (rgb.g as f32)
                < self
                    .green_luminance_threshold
                    .evaluate_at(interpolation_argument)
        {
            Intensity::Low
        } else if green_chromaticity
            > self
                .upper_green_chromaticity_threshold
                .evaluate_at(interpolation_argument)
        {
            Intensity::High
        } else {
            Intensity::Medium
        }
    }
}
