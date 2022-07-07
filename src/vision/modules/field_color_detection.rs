use module_derive::module;
use types::FieldColor;

pub struct FieldColorDetection;

#[module(vision)]
#[main_output(data_type = FieldColor)]
impl FieldColorDetection {}

impl FieldColorDetection {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs {
            field_color: Some(FieldColor {
                red_chromaticity_threshold: 0.37,
                blue_chromaticity_threshold: 0.38,
                lower_green_chromaticity_threshold: 0.4,
                upper_green_chromaticity_threshold: 0.43,
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
