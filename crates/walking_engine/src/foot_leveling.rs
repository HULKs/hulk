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
        let robot_orientation = *context.robot_orientation;
        let robot_to_walk_rotation = context.robot_to_walk.rotation();
        let level_orientation = robot_orientation.inner * robot_to_walk_rotation.inner.inverse();
        let (level_roll, level_pitch, _) = level_orientation.euler_angles();

        let return_factor =
            if normalized_time_since_start < context.parameters.start_level_reduce_to_zero {
                1.0
            } else {
                1.0 - normalized_time_since_start
            };

        let pitch_base_factor = if level_pitch > 0.0 {
            context.parameters.pitch_positive_level_factor
        } else {
            context.parameters.pitch_negative_level_factor
        };

        let pitch_scale_factor =
            (level_pitch.abs() / context.parameters.pitch_level_scale).min(1.0);
        let target_pitch = -level_pitch * return_factor * pitch_base_factor * pitch_scale_factor;

        let roll_scale_factor = (level_roll.abs() / context.parameters.roll_level_scale).min(1.0);
        let target_roll =
            -level_roll * return_factor * context.parameters.roll_level_factor * roll_scale_factor;

        let max_delta = context.parameters.max_level_delta;

        self.roll = self.roll + (target_roll - self.roll).clamp(-max_delta, max_delta);
        self.pitch = self.pitch + (target_pitch - self.pitch).clamp(-max_delta, max_delta);
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
