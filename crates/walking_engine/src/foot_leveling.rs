use coordinate_systems::Robot;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{joints::body::LowerBodyJoints, support_foot::Side};

use crate::Context;

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Default,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct FootLeveling {
    pub roll: f32,
    pub pitch: f32,
}

impl FootLeveling {
    pub fn tick(&mut self, context: &Context, normalized_time_since_start: f32) {
        let parameters = &context.parameters.foot_leveling;

        // The default torso rotation represents the desired, slightly leaned forward/backward configuration
        let robot_to_walk = context.robot_to_walk.rotation();

        let current_orientation = context.robot_orientation.rotation::<Robot>();

        let walk_to_field = current_orientation * robot_to_walk.inverse();
        let leveling_error = walk_to_field.inverse().as_orientation();

        let ([pitch_angle, roll_angle, _], _) = leveling_error
            .inner
            .to_rotation_matrix()
            .euler_angles_ordered(
                [
                    nalgebra::Vector3::y_axis(),
                    nalgebra::Vector3::x_axis(),
                    nalgebra::Vector3::z_axis(),
                ],
                false,
            );

        // Use a full effect early in the step, then reduce the leveling effect gradually over the step progress
        let leveling_factor = if normalized_time_since_start < parameters.start_reduce_to_zero {
            1.0
        } else {
            1.0 - normalized_time_since_start
        };

        // Choose the base pitch factor depending on whether the robot is leaning forward or backward
        let base_pitch_factor = if pitch_angle < 0.0 {
            parameters.leaning_forward_factor
        } else {
            parameters.leaning_backwards_factor
        };

        let pitch_scaling = (pitch_angle.abs() / parameters.pitch_scale).min(1.0);
        let desired_pitch_diff = pitch_angle * leveling_factor * base_pitch_factor * pitch_scaling;

        let base_roll_factor = parameters.roll_factor;
        let roll_scaling = (roll_angle.abs() / parameters.roll_scale).min(1.0);
        let desired_roll_diff = roll_angle * leveling_factor * base_roll_factor * roll_scaling;

        // Smoothly update the corrections with a maximum allowed delta.
        let max_delta = parameters.max_level_delta;
        self.roll += (desired_roll_diff - self.roll).clamp(-max_delta, max_delta);
        self.pitch += (desired_pitch_diff - self.pitch).clamp(-max_delta, max_delta);

        // Limit roll and pitch
        self.roll = self.roll.clamp(-parameters.max_roll, parameters.max_roll);
        self.pitch = self
            .pitch
            .clamp(-parameters.max_pitch, parameters.max_pitch);
    }
}

pub trait FootLevelingExt {
    fn level_swing_foot(self, state: &FootLeveling, support_side: Side) -> Self;
}

impl FootLevelingExt for LowerBodyJoints {
    fn level_swing_foot(mut self, state: &FootLeveling, support_side: Side) -> Self {
        let swing_leg = match support_side {
            Side::Left => &mut self.right_leg,
            Side::Right => &mut self.left_leg,
        };
        swing_leg.ankle_roll += state.roll;
        swing_leg.ankle_pitch += state.pitch;
        self
    }
}
