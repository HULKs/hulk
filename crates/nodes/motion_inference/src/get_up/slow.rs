use ::kinematics::joints::Joints;
use coordinate_systems::Robot;
use linear_algebra::Vector3;
use ros_z::time::Time;
use types::joint_limits::JointLimits;

use super::GetUp;
use crate::{
    config::{Policy, clip_measurement},
    observation::SensorFrame,
};

pub struct Observation {
    pub gravity: Vector3<Robot>,
    pub angular_velocity: Vector3<Robot>,
    pub progress: f32,
    joint_velocity_scale: f32,
    pub position_offsets: Joints<f32>,
    pub joint_velocity: Joints<f32>,
    pub previous_target_offsets: Joints<f32>,
}

impl Observation {
    pub fn new(
        state: &GetUp,
        sensor: &SensorFrame,
        joint_velocity: &Joints<f32>,
        now: Time,
        joints: &JointLimits,
    ) -> Self {
        let offset = Policy::SlowGetUp.offset(&state.parameters);
        Self {
            joint_velocity_scale: state.parameters.observation.joint_velocity_scale,
            gravity: Vector3::wrap(sensor.gravity().into()),
            angular_velocity: sensor.gyro,
            progress: now.duration_since(state.start).as_secs_f32()
                * state.parameters.get_up.progress_rate
                / state.reference_duration_seconds,
            position_offsets: clip_measurement(sensor.position, joints.position) - offset,
            joint_velocity: *joint_velocity,
            previous_target_offsets: sensor.last_commanded_position - offset,
        }
    }

    pub fn to_tensor(&self) -> [f32; 73] {
        let mut input = [0.0; 73];
        input[..3].copy_from_slice(self.gravity.inner.as_slice());
        input[3..6].copy_from_slice(self.angular_velocity.inner.as_slice());
        input[6] = self.progress.clamp(0.0, 1.0);
        for (slot, value) in input[7..29].iter_mut().zip(self.position_offsets) {
            *slot = value;
        }
        for (slot, value) in input[29..51].iter_mut().zip(self.joint_velocity) {
            *slot = value * self.joint_velocity_scale;
        }
        // Includes the head, unlike the locomotion policies' previous-target features.
        for (slot, value) in input[51..73].iter_mut().zip(self.previous_target_offsets) {
            *slot = value;
        }
        input
    }
}
