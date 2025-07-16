use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{
    joints::{body::LowerBodyJoints, leg::LegJoints},
    support_foot::Side,
};

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
pub struct GyroBalancing {
    balancing: LegJoints,
}

impl GyroBalancing {
    pub fn tick(&mut self, context: &Context) {
        let gyro = context.gyro;
        let parameters = &context.parameters.gyro_balancing;
        let factors = &parameters.balance_factors;

        let roll = gyro.x;
        let roll_scaling = (roll.abs() / parameters.noise_scale.x).min(1.0);
        let pitch = gyro.y;
        let pitch_scaling = (pitch.abs() / parameters.noise_scale.y).min(1.0);

        let support_balancing = LegJoints {
            ankle_pitch: factors.ankle_pitch * pitch * pitch_scaling,
            ankle_roll: factors.ankle_roll * roll * roll_scaling,
            hip_pitch: factors.hip_pitch * pitch * pitch_scaling,
            hip_roll: factors.hip_roll * roll * roll_scaling,
            hip_yaw_pitch: 0.0,
            knee_pitch: factors.knee_pitch * pitch * pitch_scaling,
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
        *support_leg += state.balancing;
        self
    }
}
