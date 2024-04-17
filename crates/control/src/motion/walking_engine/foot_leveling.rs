use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{joints::body::LowerBodyJoints, support_foot::Side};

use super::CycleContext;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, SerializeHierarchy)]
pub struct FootLeveling {
    pub roll: f32,
    pub pitch: f32,
}

impl FootLeveling {
    pub fn tick(&mut self, context: &CycleContext, normalized_time_since_start: f32) {
        let target_roll = -context.sensor_data.inertial_measurement_unit.roll_pitch.x
            * (1.0 - normalized_time_since_start);
        let target_pitch = -context.sensor_data.inertial_measurement_unit.roll_pitch.y
            * (1.0 - normalized_time_since_start);

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
