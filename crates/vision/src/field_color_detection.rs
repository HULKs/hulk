use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use linear_algebra::Isometry2;
use types::{
    coordinate_systems::{Field, Ground},
    field_color::FieldColor,
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
    blue_chromaticity_threshold: Parameter<
        Interpolated,
        "field_color_detection.$cycler_instance.blue_chromaticity_threshold",
    >,
    green_luminance_threshold:
        Parameter<Interpolated, "field_color_detection.$cycler_instance.green_luminance_threshold">,
    lower_green_chromaticity_threshold: Parameter<
        Interpolated,
        "field_color_detection.$cycler_instance.lower_green_chromaticity_threshold",
    >,
    red_chromaticity_threshold: Parameter<
        Interpolated,
        "field_color_detection.$cycler_instance.red_chromaticity_threshold",
    >,
    upper_green_chromaticity_threshold: Parameter<
        Interpolated,
        "field_color_detection.$cycler_instance.upper_green_chromaticity_threshold",
    >,

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
                red_chromaticity_threshold: context
                    .red_chromaticity_threshold
                    .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half),
                blue_chromaticity_threshold: context
                    .blue_chromaticity_threshold
                    .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half),
                lower_green_chromaticity_threshold: context
                    .lower_green_chromaticity_threshold
                    .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half),
                upper_green_chromaticity_threshold: context
                    .upper_green_chromaticity_threshold
                    .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half),
                green_luminance_threshold: context
                    .green_luminance_threshold
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
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
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
