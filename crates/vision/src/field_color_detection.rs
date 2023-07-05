use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::Isometry2;
use types::{interpolated::Interpolated, FieldColor};

pub struct FieldColorDetection {
    fallback_robot_to_field_of_home_after_coin_toss_before_second_half: Isometry2<f32>,
}

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

    pub robot_to_field_of_home_after_coin_toss_before_second_half: Input<
        Option<Isometry2<f32>>,
        "Control",
        "robot_to_field_of_home_after_coin_toss_before_second_half?",
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
            fallback_robot_to_field_of_home_after_coin_toss_before_second_half: Isometry2::default(
            ),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let robot_to_field_of_home_after_coin_toss_before_second_half = context
            .robot_to_field_of_home_after_coin_toss_before_second_half
            .copied()
            .unwrap_or(self.fallback_robot_to_field_of_home_after_coin_toss_before_second_half);
        self.fallback_robot_to_field_of_home_after_coin_toss_before_second_half =
            robot_to_field_of_home_after_coin_toss_before_second_half;

        Ok(MainOutputs {
            field_color: FieldColor {
                red_chromaticity_threshold: context
                    .red_chromaticity_threshold
                    .evaluate_at(robot_to_field_of_home_after_coin_toss_before_second_half),
                blue_chromaticity_threshold: context
                    .blue_chromaticity_threshold
                    .evaluate_at(robot_to_field_of_home_after_coin_toss_before_second_half),
                lower_green_chromaticity_threshold: context
                    .lower_green_chromaticity_threshold
                    .evaluate_at(robot_to_field_of_home_after_coin_toss_before_second_half),
                upper_green_chromaticity_threshold: context
                    .upper_green_chromaticity_threshold
                    .evaluate_at(robot_to_field_of_home_after_coin_toss_before_second_half),
                green_luminance_threshold: context
                    .green_luminance_threshold
                    .evaluate_at(robot_to_field_of_home_after_coin_toss_before_second_half),
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
