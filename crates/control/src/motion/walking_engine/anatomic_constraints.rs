use types::{step_plan::Step, support_foot::Side};

pub trait AnatomicConstraints {
    fn clamp_to_anatomic_constraints(self, support_side: Side, maximum_inside_turn: f32) -> Step;
}

impl AnatomicConstraints for Step {
    fn clamp_to_anatomic_constraints(self, support_side: Side, maximum_inside_turn: f32) -> Step {
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
            self.turn.clamp(-maximum_inside_turn, maximum_inside_turn)
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
