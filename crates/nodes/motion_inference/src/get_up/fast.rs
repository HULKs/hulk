use ::kinematics::joints::Joints;
use coordinate_systems::Robot;
use linear_algebra::Vector3;
use types::joint_limits::JointLimits;

use super::GetUp;
use crate::{
    config::{Policy, clip_measurement},
    observation::SensorFrame,
};

pub struct Observation {
    pub angular_velocity: Vector3<Robot>,
    pub gravity: Vector3<Robot>,
    pub position_offsets: Joints<f32>,
    pub joint_velocity: Joints<f32>,
    pub previous_action: Joints<f32>,
    joint_velocity_scale: f32,
}

impl Observation {
    pub fn new(
        state: &GetUp,
        sensor: &SensorFrame,
        joint_velocity: &Joints<f32>,
        joints: &JointLimits,
    ) -> Self {
        Self {
            joint_velocity_scale: state.parameters.observation.joint_velocity_scale,
            angular_velocity: sensor.gyro,
            gravity: Vector3::wrap(sensor.gravity().into()),
            position_offsets: clip_measurement(sensor.position, joints.position)
                - Policy::FastGetUp.offset(&state.parameters),
            joint_velocity: *joint_velocity,
            previous_action: state.previous_action,
        }
    }

    pub fn to_tensor(&self) -> [f32; 72] {
        let mut input = [0.0; 72];
        input[..3].copy_from_slice(self.angular_velocity.inner.as_slice());
        input[3..6].copy_from_slice(self.gravity.inner.as_slice());
        for (slot, value) in input[6..28].iter_mut().zip(self.position_offsets) {
            *slot = value;
        }
        for (slot, value) in input[28..50].iter_mut().zip(self.joint_velocity) {
            *slot = value * self.joint_velocity_scale;
        }
        for (slot, value) in input[50..72].iter_mut().zip(self.previous_action) {
            *slot = value;
        }
        input
    }
}
