use context_attribute::context;
use framework::{MainOutput, Parameter};
use types::FieldColor;

pub struct FieldColorDetection {}

#[context]
pub struct NewContext {
    pub blue_chromaticity_threshold:
        Parameter<f32, "$cycler_instance/field_color_detection/blue_chromaticity_threshold">,
    pub green_luminance_threshold:
        Parameter<u8, "$cycler_instance/field_color_detection/green_luminance_threshold">,
    pub lower_green_chromaticity_threshold:
        Parameter<f32, "$cycler_instance/field_color_detection/lower_green_chromaticity_threshold">,
    pub red_chromaticity_threshold:
        Parameter<f32, "$cycler_instance/field_color_detection/red_chromaticity_threshold">,
    pub upper_green_chromaticity_threshold:
        Parameter<f32, "$cycler_instance/field_color_detection/upper_green_chromaticity_threshold">,
}

#[context]
pub struct CycleContext {
    pub blue_chromaticity_threshold:
        Parameter<f32, "$cycler_instance/field_color_detection/blue_chromaticity_threshold">,
    pub green_luminance_threshold:
        Parameter<u8, "$cycler_instance/field_color_detection/green_luminance_threshold">,
    pub lower_green_chromaticity_threshold:
        Parameter<f32, "$cycler_instance/field_color_detection/lower_green_chromaticity_threshold">,
    pub red_chromaticity_threshold:
        Parameter<f32, "$cycler_instance/field_color_detection/red_chromaticity_threshold">,
    pub upper_green_chromaticity_threshold:
        Parameter<f32, "$cycler_instance/field_color_detection/upper_green_chromaticity_threshold">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub field_color: MainOutput<FieldColor>,
}

impl FieldColorDetection {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
