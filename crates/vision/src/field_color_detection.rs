use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::FieldColor;

pub struct FieldColorDetection {}

#[context]
pub struct CreationContext {
    pub blue_chromaticity_threshold:
        Parameter<f32, "field_color_detection/$cycler_instance/blue_chromaticity_threshold">,
    pub green_luminance_threshold:
        Parameter<u8, "field_color_detection/$cycler_instance/green_luminance_threshold">,
    pub lower_green_chromaticity_threshold:
        Parameter<f32, "field_color_detection/$cycler_instance/lower_green_chromaticity_threshold">,
    pub red_chromaticity_threshold:
        Parameter<f32, "field_color_detection/$cycler_instance/red_chromaticity_threshold">,
    pub upper_green_chromaticity_threshold:
        Parameter<f32, "field_color_detection/$cycler_instance/upper_green_chromaticity_threshold">,
}

#[context]
pub struct CycleContext {
    pub blue_chromaticity_threshold:
        Parameter<f32, "field_color_detection/$cycler_instance/blue_chromaticity_threshold">,
    pub green_luminance_threshold:
        Parameter<u8, "field_color_detection/$cycler_instance/green_luminance_threshold">,
    pub lower_green_chromaticity_threshold:
        Parameter<f32, "field_color_detection/$cycler_instance/lower_green_chromaticity_threshold">,
    pub red_chromaticity_threshold:
        Parameter<f32, "field_color_detection/$cycler_instance/red_chromaticity_threshold">,
    pub upper_green_chromaticity_threshold:
        Parameter<f32, "field_color_detection/$cycler_instance/upper_green_chromaticity_threshold">,
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

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
