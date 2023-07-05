use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{interpolated::Interpolated, FieldColor};

pub struct FieldColorDetection {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub blue_chromaticity_threshold: Parameter<
        Interpolated,
        "field_color_detection.$cycler_instance.blue_chromaticity_threshold",
    >,
    pub green_luminance_threshold:
        Parameter<Interpolated, "field_color_detection.$cycler_instance.green_luminance_threshold">,
    pub lower_green_chromaticity_threshold: Parameter<
        Interpolated,
        "field_color_detection.$cycler_instance.lower_green_chromaticity_threshold",
    >,
    pub red_chromaticity_threshold: Parameter<
        Interpolated,
        "field_color_detection.$cycler_instance.red_chromaticity_threshold",
    >,
    pub upper_green_chromaticity_threshold: Parameter<
        Interpolated,
        "field_color_detection.$cycler_instance.upper_green_chromaticity_threshold",
    >,
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
    use nalgebra::Point2;
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
            red_chromaticity_threshold: 0.37.into(),
            blue_chromaticity_threshold: 0.38.into(),
            lower_green_chromaticity_threshold: 0.4.into(),
            upper_green_chromaticity_threshold: 0.43.into(),
            green_luminance_threshold: 255.0.into(),
        };
        let field_color_intensity = field_color.get_intensity(ycbcr, Point2::origin());
        assert_eq!(field_color_intensity, Intensity::High);
        let ycbcr = YCbCr444 {
            y: 128,
            cb: 255,
            cr: 0,
        };
        let field_color_intensity = field_color.get_intensity(ycbcr, Point2::origin());
        assert_eq!(field_color_intensity, Intensity::Low);
    }
}
