use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::color::{Intensity, Rgb, RgbChannel, YCbCr444};

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct FieldColor {
    pub red_chromaticity_threshold: f32,
    pub blue_chromaticity_threshold: f32,
    pub green_chromaticity_threshold: f32,
    pub green_luminance_threshold: u8,
    pub luminance_threshold: u8,
}

impl FieldColor {
    pub fn get_intensity(&self, color: YCbCr444) -> Intensity {
        let rgb = Rgb::from(color);

        let red_chromaticity = rgb.get_chromaticity(RgbChannel::Red);
        let green_chromaticity = rgb.get_chromaticity(RgbChannel::Green);
        let blue_chromaticity = rgb.get_chromaticity(RgbChannel::Blue);
        if (red_chromaticity > self.red_chromaticity_threshold
            || blue_chromaticity > self.blue_chromaticity_threshold
            || green_chromaticity < self.green_chromaticity_threshold
            || (rgb.g) < self.green_luminance_threshold)
            && (rgb.get_luminance()) > self.luminance_threshold
        {
            Intensity::Low
        } else {
            Intensity::High
        }
    }
}
