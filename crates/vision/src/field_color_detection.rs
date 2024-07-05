use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::MainOutput;
use linear_algebra::Isometry2;
use types::{
    field_color::{FieldColor, FieldColorFunction},
    interpolated::Interpolated,
};

#[derive(Deserialize, Serialize)]
pub struct FieldColorDetection {
    ground_to_field_of_home_after_coin_toss_before_second_half: Isometry2<Ground, Field>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    function: Parameter<FieldColorFunction, "field_color_detection.function">,
    luminance_threshold:
        Parameter<Interpolated, "field_color_detection.$cycler_instance.luminance_threshold">,
    blue_chromaticity_threshold: Parameter<
        Interpolated,
        "field_color_detection.$cycler_instance.blue_chromaticity_threshold",
    >,
    green_luminance_threshold:
        Parameter<Interpolated, "field_color_detection.$cycler_instance.green_luminance_threshold">,
    red_chromaticity_threshold: Parameter<
        Interpolated,
        "field_color_detection.$cycler_instance.red_chromaticity_threshold",
    >,
    green_chromaticity_threshold: Parameter<
        Interpolated,
        "field_color_detection.$cycler_instance.green_chromaticity_threshold",
    >,
    hue_low_threshold:
        Parameter<Interpolated, "field_color_detection.$cycler_instance.hue_low_threshold">,
    hue_high_threshold:
        Parameter<Interpolated, "field_color_detection.$cycler_instance.hue_high_threshold">,
    saturation_low_threshold:
        Parameter<Interpolated, "field_color_detection.$cycler_instance.saturation_low_threshold">,
    saturation_high_threshold:
        Parameter<Interpolated, "field_color_detection.$cycler_instance.saturation_high_threshold">,

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
                function: *context.function,
                luminance_threshold: context
                    .luminance_threshold
                    .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half),
                red_chromaticity_threshold: context
                    .red_chromaticity_threshold
                    .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half),
                blue_chromaticity_threshold: context
                    .blue_chromaticity_threshold
                    .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half),
                green_chromaticity_threshold: context
                    .green_chromaticity_threshold
                    .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half),
                green_luminance_threshold: context
                    .green_luminance_threshold
                    .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half),
                hue_low_threshold: context
                    .hue_low_threshold
                    .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half),
                hue_high_threshold: context
                    .hue_high_threshold
                    .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half),
                saturation_low_threshold: context
                    .saturation_low_threshold
                    .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half),
                saturation_high_threshold: context
                    .saturation_high_threshold
                    .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half),
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
            function: FieldColorFunction::GreenChromaticity,
            luminance_threshold: 25.0,
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            hue_low_threshold: 0.0,
            hue_high_threshold: 360.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 255.0,
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
