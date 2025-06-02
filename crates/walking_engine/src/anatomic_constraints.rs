use coordinate_systems::Walk;
use linear_algebra::{point, Pose2};
use types::{step::Step, support_foot::Side};

use crate::{feet::Feet, parameters::Parameters};

pub trait AnatomicConstraints {
    fn clamp_to_anatomic_constraints(
        self,
        support_side: Side,
        base_maximum_inside_turn: f32,
        maximum_inside_turn_increase: f32,
    ) -> Step;
}

impl AnatomicConstraints for Step {
    fn clamp_to_anatomic_constraints(
        self,
        support_side: Side,
        base_maximum_inside_turn: f32,
        maximum_inside_turn_increase: f32,
    ) -> Step {
        let sideways_direction = if self.left.is_sign_positive() {
            Side::Left
        } else {
            Side::Right
        };
        let clamped_left = if sideways_direction == support_side {
            0.0
        } else {
            self.left
        };
        let turn_direction = if self.turn.is_sign_positive() {
            Side::Left
        } else {
            Side::Right
        };
        let clamped_turn = if turn_direction == support_side {
            self.turn.clamp(
                -base_maximum_inside_turn - maximum_inside_turn_increase * self.left.abs(),
                base_maximum_inside_turn + maximum_inside_turn_increase * self.left.abs(),
            )
        } else {
            self.turn
        };
        Step {
            forward: self.forward,
            left: clamped_left,
            turn: clamped_turn,
        }
    }
}

pub fn clamp_feet_to_anatomic_constraints(
    feet: Feet<Pose2<Walk>>,
    support_side: Side,
    parameters: &Parameters,
) -> Feet<Pose2<Walk>> {
    let (left, right) = match support_side {
        Side::Left => (feet.support_sole, feet.swing_sole),
        Side::Right => (feet.swing_sole, feet.support_sole),
    };
    let left_base_offset = parameters.base.foot_offset_left;
    let right_base_offset = parameters.base.foot_offset_right;
    let valid_x_range = -0.05..0.05;
    let left_valid_y_range = left_base_offset.y()..0.1;
    let right_valid_y_range = -0.1..right_base_offset.y();

    let clamped_left = Pose2::from_parts(
        point![
            left.position()
                .x()
                .clamp(valid_x_range.start, valid_x_range.end),
            left.position()
                .y()
                .clamp(left_valid_y_range.start, left_valid_y_range.end),
        ],
        left.orientation(),
    );

    let clamped_right = Pose2::from_parts(
        point![
            right
                .position()
                .x()
                .clamp(valid_x_range.start, valid_x_range.end),
            right
                .position()
                .y()
                .clamp(right_valid_y_range.start, right_valid_y_range.end),
        ],
        right.orientation(),
    );

    match support_side {
        Side::Left => Feet {
            support_sole: clamped_left,
            swing_sole: clamped_right,
        },
        Side::Right => Feet {
            support_sole: clamped_right,
            swing_sole: clamped_left,
        },
    }
}
