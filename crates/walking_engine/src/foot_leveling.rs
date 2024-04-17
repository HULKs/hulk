use coordinate_systems::Robot;
use linear_algebra::Orientation3;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{joints::body::LowerBodyJoints, support_foot::Side};

use crate::Context;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, SerializeHierarchy)]
pub struct FootLeveling {
    pub roll: f32,
    pub pitch: f32,
}

impl FootLeveling {
    pub fn tick(&mut self, context: &Context, normalized_time_since_start: f32) {
        let imu_roll_pitch = context.sensor_data.inertial_measurement_unit.roll_pitch;
        let imu_orientation =
            Orientation3::<Robot>::from_euler_angles(imu_roll_pitch.x, imu_roll_pitch.y, 0.0)
                .mirror();
        let level_orientation = context.robot_to_walk.rotation() * imu_orientation;

        let (level_roll, level_pitch, _) = level_orientation.inner.euler_angles();
        let target_roll = level_pitch * (1.0 - normalized_time_since_start);
        let target_pitch = level_roll * (1.0 - normalized_time_since_start);

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
