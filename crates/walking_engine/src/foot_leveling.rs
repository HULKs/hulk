use coordinate_systems::Robot;
use kinematics::forward;
use linear_algebra::IntoTransform;
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
    pub fn tick(
        &mut self,
        context: &Context,
        support_side: Side,
        normalized_time_since_start: f32,
    ) {
        let parameters = &context.parameters.foot_leveling;
        let current_orientation = context.robot_orientation.rotation::<Robot>();

        let (roll_angle, pitch_angle, _) = match support_side {
            Side::Left => {
                let right_sole_to_field = current_orientation
                    * forward::right_sole_to_robot(&context.last_actuated_joints.right_leg);

                right_sole_to_field.as_pose().orientation().euler_angles()
            }
            Side::Right => {
                let left_sole_to_field = current_orientation
                    * forward::left_sole_to_robot(&context.last_actuated_joints.left_leg);

                left_sole_to_field.as_pose().orientation().euler_angles()
            }
        };

        // Use a full effect early in the step, then reduce the leveling effect gradually over the step progress
        let leveling_factor = if normalized_time_since_start < parameters.start_reduce_to_zero {
            1.0
        } else {
            1.0 - normalized_time_since_start
        };

        // Choose the base pitch factor depending on whether the robot is leaning forward or backward
        let base_pitch_factor = if pitch_angle > 0.0 {
            parameters.leaning_forward_factor
        } else {
            parameters.leaning_backwards_factor
        };

        let pitch_scaling = (pitch_angle.abs() / parameters.pitch_scale).min(1.0);
        let desired_pitch = -pitch_angle * leveling_factor * base_pitch_factor * pitch_scaling;

        let base_roll_factor = parameters.roll_factor;
        let roll_scaling = (roll_angle.abs() / parameters.roll_scale).min(1.0);
        let desired_roll = -roll_angle * leveling_factor * base_roll_factor * roll_scaling;

        // Smoothly update the corrections with a maximum allowed delta.
        let max_delta = parameters.max_level_delta;
        self.roll += (desired_roll - self.roll).clamp(-max_delta, max_delta);
        self.pitch += (desired_pitch - self.pitch).clamp(-max_delta, max_delta);
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
