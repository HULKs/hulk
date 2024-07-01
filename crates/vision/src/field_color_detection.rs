use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::MainOutput;
use linear_algebra::Isometry2;
use types::field_color::FieldColor;

#[derive(Deserialize, Serialize)]
pub struct FieldColorDetection {
    ground_to_field_of_home_after_coin_toss_before_second_half: Isometry2<Ground, Field>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    blue_chromaticity_threshold:
        Parameter<f32, "field_color_detection.$cycler_instance.blue_chromaticity_threshold">,
    green_luminance_threshold:
        Parameter<u8, "field_color_detection.$cycler_instance.green_luminance_threshold">,
    red_chromaticity_threshold:
        Parameter<f32, "field_color_detection.$cycler_instance.red_chromaticity_threshold">,
    green_chromaticity_threshold:
        Parameter<f32, "field_color_detection.$cycler_instance.green_chromaticity_threshold">,
    luminance_threshold:
        Parameter<u8, "field_color_detection.$cycler_instance.luminance_threshold">,

    ground_to_field_of_home_after_coin_toss_before_second_half: Input<
        Option<Isometry2<Ground, Field>>,
        "Control",
        "ground_to_field_of_home_after_coin_toss_before_second_half?",
    >,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub field_color: MainOutput<FieldColor>,
}

impl FieldColorDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            ground_to_field_of_home_after_coin_toss_before_second_half: Isometry2::identity(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        if let Some(ground_to_field_of_home_after_coin_toss_before_second_half) =
            context.ground_to_field_of_home_after_coin_toss_before_second_half
        {
            self.ground_to_field_of_home_after_coin_toss_before_second_half =
                *ground_to_field_of_home_after_coin_toss_before_second_half;
        }

        Ok(MainOutputs {
            field_color: FieldColor {
                red_chromaticity_threshold: *context.red_chromaticity_threshold,
                blue_chromaticity_threshold: *context.blue_chromaticity_threshold,
                green_chromaticity_threshold: *context.green_chromaticity_threshold,
                green_luminance_threshold: *context.green_luminance_threshold,
                luminance_threshold: *context.luminance_threshold,
            }
            .into(),
        })
    }
}

#[cfg(test)]
mod test {
    use types::color::{Intensity, YCbCr444};

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
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255,
            luminance_threshold: 25,
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
