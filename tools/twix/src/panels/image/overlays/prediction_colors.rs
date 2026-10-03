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
