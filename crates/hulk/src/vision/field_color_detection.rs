use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::FieldColor;

pub struct FieldColorDetection {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub blue_chromaticity_threshold:
        Parameter<f32, "field_color_detection.$cycler_instance.blue_chromaticity_threshold">,
    pub green_luminance_threshold:
        Parameter<u8, "field_color_detection.$cycler_instance.green_luminance_threshold">,
    pub lower_green_chromaticity_threshold:
        Parameter<f32, "field_color_detection.$cycler_instance.lower_green_chromaticity_threshold">,
    pub red_chromaticity_threshold:
        Parameter<f32, "field_color_detection.$cycler_instance.red_chromaticity_threshold">,
    pub upper_green_chromaticity_threshold:
        Parameter<f32, "field_color_detection.$cycler_instance.upper_green_chromaticity_threshold">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub field_color: MainOutput<FieldColor>,
}

impl FieldColorDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs {
            field_color: FieldColor {
                red_chromaticity_threshold: *context.red_chromaticity_threshold,
                blue_chromaticity_threshold: *context.blue_chromaticity_threshold,
                lower_green_chromaticity_threshold: *context.lower_green_chromaticity_threshold,
                upper_green_chromaticity_threshold: *context.upper_green_chromaticity_threshold,
                green_luminance_threshold: *context.green_luminance_threshold,
            }
            .into(),
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
