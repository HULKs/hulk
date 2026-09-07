use eframe::egui::Color32;
use types::object_detection::RobocupObjectLabel;

// Based on the presentation palette, with colliding hues separated for live overlays.
const BALL: Color32 = Color32::from_rgb(255, 140, 56);
const GOAL_POST: Color32 = Color32::from_rgb(66, 190, 80);
const L_SPOT: Color32 = Color32::from_rgb(67, 112, 255);
const PENALTY_SPOT: Color32 = Color32::from_rgb(110, 40, 170);
const ROBOT: Color32 = Color32::from_rgb(255, 225, 25);
const T_SPOT: Color32 = Color32::from_rgb(40, 202, 255);
const X_SPOT: Color32 = Color32::from_rgb(185, 35, 35);
pub(super) const PERSON_POSE: Color32 = Color32::from_rgb(255, 100, 190);

pub(super) const fn robocup_object(label: RobocupObjectLabel) -> Color32 {
    match label {
        RobocupObjectLabel::Ball => BALL,
        RobocupObjectLabel::GoalPost => GOAL_POST,
        RobocupObjectLabel::LSpot => L_SPOT,
        RobocupObjectLabel::PenaltySpot => PENALTY_SPOT,
        RobocupObjectLabel::Robot => ROBOT,
        RobocupObjectLabel::TSpot => T_SPOT,
        RobocupObjectLabel::XSpot => X_SPOT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_is_mapped_to_the_expected_classes() {
        assert_eq!(
            [
                robocup_object(RobocupObjectLabel::Ball),
                robocup_object(RobocupObjectLabel::GoalPost),
                robocup_object(RobocupObjectLabel::LSpot),
                robocup_object(RobocupObjectLabel::PenaltySpot),
                robocup_object(RobocupObjectLabel::Robot),
                robocup_object(RobocupObjectLabel::TSpot),
                robocup_object(RobocupObjectLabel::XSpot),
                PERSON_POSE,
            ],
            [
                Color32::from_rgb(255, 140, 56),
                Color32::from_rgb(66, 190, 80),
                Color32::from_rgb(67, 112, 255),
                Color32::from_rgb(110, 40, 170),
                Color32::from_rgb(255, 225, 25),
                Color32::from_rgb(40, 202, 255),
                Color32::from_rgb(185, 35, 35),
                Color32::from_rgb(255, 100, 190),
            ]
        );
    }

    #[test]
    fn class_colors_are_perceptually_distinct() {
        const MINIMUM_OKLAB_DISTANCE: f32 = 0.18;
        let colors = [
            BALL,
            GOAL_POST,
            L_SPOT,
            PENALTY_SPOT,
            ROBOT,
            T_SPOT,
            X_SPOT,
            PERSON_POSE,
        ];

        for (index, first) in colors.iter().enumerate() {
            for second in &colors[index + 1..] {
                assert!(
                    oklab_distance(*first, *second) >= MINIMUM_OKLAB_DISTANCE,
                    "class colors {first:?} and {second:?} are too similar",
                );
            }
        }
    }

    fn oklab_distance(first: Color32, second: Color32) -> f32 {
        let first = oklab(first);
        let second = oklab(second);
        ((first[0] - second[0]).powi(2)
            + (first[1] - second[1]).powi(2)
            + (first[2] - second[2]).powi(2))
        .sqrt()
    }

    fn oklab(color: Color32) -> [f32; 3] {
        let [red, green, blue, _] = color.to_srgba_unmultiplied();
        let [red, green, blue] = [red, green, blue].map(|channel| {
            let channel = channel as f32 / 255.0;
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        });
        let lightness = (0.412_221_46 * red + 0.536_332_55 * green + 0.051_445_995 * blue).cbrt();
        let medium = (0.211_903_5 * red + 0.680_699_5 * green + 0.107_396_96 * blue).cbrt();
        let short = (0.088_302_46 * red + 0.281_718_85 * green + 0.629_978_7 * blue).cbrt();

        [
            0.210_454_26 * lightness + 0.793_617_8 * medium - 0.004_072_047 * short,
            1.977_998_5 * lightness - 2.428_592_2 * medium + 0.450_593_7 * short,
            0.025_904_037 * lightness + 0.782_771_77 * medium - 0.808_675_77 * short,
        ]
    }
}
