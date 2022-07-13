use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::{Intensity, Rgb, RgbChannel, YCbCr444};

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FieldColor {
    pub red_chromaticity_threshold: f32,
    pub blue_chromaticity_threshold: f32,
    pub lower_green_chromaticity_threshold: f32,
    pub upper_green_chromaticity_threshold: f32,
    pub green_luminance_threshold: u8,
}

impl FieldColor {
    pub fn get_intensity(&self, color: YCbCr444) -> Intensity {
        let rgb = Rgb::from(color);
        let red_chromaticity = rgb.get_chromaticity(RgbChannel::Red);
        let green_chromaticity = rgb.get_chromaticity(RgbChannel::Green);
        let blue_chromaticity = rgb.get_chromaticity(RgbChannel::Blue);
        if red_chromaticity > self.red_chromaticity_threshold
            || blue_chromaticity > self.blue_chromaticity_threshold
            || green_chromaticity < self.lower_green_chromaticity_threshold
            || rgb.g < self.green_luminance_threshold
        {
            Intensity::Low
        } else if green_chromaticity > self.upper_green_chromaticity_threshold {
            Intensity::High
        } else {
            Intensity::Medium
        }
    }
}
