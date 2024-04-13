use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{
    joints::{body::LowerBodyJoints, leg::LegJoints},
    support_foot::Side,
};

use super::CycleContext;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, SerializeHierarchy)]
pub struct GyroBalancing {
    balancing: LegJoints,
}

impl GyroBalancing {
    pub fn tick(&mut self, context: &CycleContext, gyro: nalgebra::Vector3<f32>) {
        let parameters = &context.parameters.gyro_balancing;
        let factors = &parameters.balance_factors;

        let support_balancing = LegJoints {
            ankle_pitch: factors.ankle_pitch * gyro.y,
            ankle_roll: factors.ankle_roll * gyro.x,
            hip_pitch: factors.hip_pitch * gyro.y,
            hip_roll: factors.hip_roll * gyro.x,
            hip_yaw_pitch: 0.0,
            knee_pitch: factors.knee_pitch * gyro.y,
        };

        let max_delta = parameters.max_delta;
        self.balancing =
            self.balancing + (support_balancing - self.balancing).clamp(-max_delta, max_delta);
    }
}

pub trait GyroBalancingExt {
    fn balance_using_gyro(self, state: &GyroBalancing, support_side: Side) -> Self;
}

impl GyroBalancingExt for LowerBodyJoints {
    fn balance_using_gyro(mut self, state: &GyroBalancing, support_side: Side) -> Self {
        let support_leg = match support_side {
            Side::Left => &mut self.left_leg,
            Side::Right => &mut self.right_leg,
        };
        *support_leg = *support_leg + state.balancing;
        self
    }
}
