use module_derive::module;
use types::FieldColor;

pub struct FieldColorDetection;

#[module(vision)]
#[parameter(path = $this_cycler.field_color_detection.red_chromaticity_threshold, data_type = f32)]
#[parameter(path = $this_cycler.field_color_detection.blue_chromaticity_threshold, data_type = f32)]
#[parameter(path = $this_cycler.field_color_detection.lower_green_chromaticity_threshold, data_type = f32)]
#[parameter(path = $this_cycler.field_color_detection.upper_green_chromaticity_threshold, data_type = f32)]
#[parameter(path = $this_cycler.field_color_detection.green_luminance_threshold, data_type = u8)]
#[main_output(data_type = FieldColor)]
impl FieldColorDetection {}

impl FieldColorDetection {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs {
            field_color: Some(FieldColor {
                red_chromaticity_threshold: *context.red_chromaticity_threshold,
                blue_chromaticity_threshold: *context.blue_chromaticity_threshold,
                lower_green_chromaticity_threshold: *context.lower_green_chromaticity_threshold,
                upper_green_chromaticity_threshold: *context.upper_green_chromaticity_threshold,
                green_luminance_threshold: *context.green_luminance_threshold,
            }),
        })
    }
}

#[cfg(test)]
mod test {
    use types::{Intensity, YCbCr444};

    use super::*;

    #[test]
    fn calculate_field_color() {
        let ycbcr = YCbCr444 {
            y: 128,
            cb: 0,
            cr: 0,
        };
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255,
        };
        let field_color_intensity = field_color.get_intensity(ycbcr);
        assert_eq!(field_color_intensity, Intensity::High);
        let ycbcr = YCbCr444 {
            y: 128,
            cb: 255,
            cr: 0,
        };
        let field_color_intensity = field_color.get_intensity(ycbcr);
        assert_eq!(field_color_intensity, Intensity::Low);
    }
}
