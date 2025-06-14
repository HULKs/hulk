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

        let rotation = match support_side {
            Side::Left => {
                // let angles = &context.measured_joints.right_leg;
                let angles = &context.last_actuated_joints.right_leg;

                let leg_orientation = forward::right_pelvis_to_robot(angles)
                    * forward::right_hip_to_right_pelvis(angles)
                    * forward::right_thigh_to_right_hip(angles)
                    * forward::right_tibia_to_right_thigh(angles);

                let right_sole_to_field = current_orientation * leg_orientation;

                right_sole_to_field.inner.rotation
            }
            Side::Right => {
                // let angles = &context.measured_joints.left_leg;
                let angles = &context.last_actuated_joints.left_leg;

                let leg_orientation = forward::left_pelvis_to_robot(angles)
                    * forward::left_hip_to_left_pelvis(angles)
                    * forward::left_thigh_to_left_hip(angles)
                    * forward::left_tibia_to_left_thigh(angles);

                let left_sole_to_field = current_orientation * leg_orientation;

                left_sole_to_field.inner.rotation
            }
        };

        let ([desired_pitch, desired_roll, _], _) = rotation
            .inverse()
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

        let base_pitch_factor = if desired_pitch < 0.0 {
            parameters.leaning_forward_factor
        } else {
            parameters.leaning_backwards_factor
        };

        // let pitch_scaling = (desired_pitch.abs() / parameters.pitch_scale).min(1.0);
        // let desired_pitch = (desired_pitch + desired_pitch * base_pitch_factor) * leveling_factor;
        //
        // let base_roll_factor = parameters.roll_factor;
        // let roll_scaling = (desired_roll.abs() / parameters.roll_scale).min(1.0);
        // let desired_roll = desired_roll * leveling_factor * base_roll_factor * roll_scaling;

        let max_delta = parameters.max_level_delta;
        // self.pitch += (desired_pitch - self.pitch).clamp(-max_delta, max_delta);
        // self.roll += (desired_roll - self.roll).clamp(-max_delta, max_delta);
        self.pitch = desired_pitch;
        self.roll = desired_roll;
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
        swing_leg.ankle_roll = state.roll;
        swing_leg.ankle_pitch = state.pitch;
        self
    }
}
