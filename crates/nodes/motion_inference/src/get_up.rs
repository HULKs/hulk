use kinematics::joints::Joints;
use ros_z::time::Time;
use types::joint_limits::JointLimits;
use types::motor_command::MotorCommand;

use crate::{
    config::{JOINT_COUNT, Parameters, Policy, clip_measurement},
    inference::position_targets,
    observation::SensorFrame,
};

pub mod fast;
pub mod slow;

pub struct GetUp {
    parameters: std::sync::Arc<Parameters>,
    start: Time,
    reference_duration_seconds: f32,
    previous_action: Joints<f32>,
}

impl GetUp {
    pub fn new(sensor: &SensorFrame, now: Time, parameters: std::sync::Arc<Parameters>) -> Self {
        let front = sensor.rotation().euler_angles().1 > 0.0;
        Self {
            reference_duration_seconds: if front {
                parameters.get_up.front_duration_seconds
            } else {
                parameters.get_up.back_duration_seconds
            },
            parameters,
            start: now,
            previous_action: Joints::fill(0.0),
        }
    }

    pub fn progress(&self, now: Time) -> f32 {
        (now.duration_since(self.start).as_secs_f32() * self.parameters.get_up.progress_rate
            / self.reference_duration_seconds)
            .clamp(0.0, 1.0)
    }

    pub(crate) fn update_parameters(&mut self, parameters: std::sync::Arc<Parameters>) {
        self.parameters = parameters;
    }

    pub fn decode(
        &mut self,
        policy: Policy,
        actions: &[f32],
        sensor: &SensorFrame,
        joints: &JointLimits,
    ) -> Joints<MotorCommand> {
        let reference_position = if policy == Policy::FastGetUp {
            clip_measurement(sensor.position, joints.position)
        } else {
            policy.offset(&self.parameters)
        };
        let action_limit = policy.action_limit(&self.parameters);
        self.previous_action = actions[..JOINT_COUNT]
            .iter()
            .map(|action| action.clamp(-action_limit, action_limit))
            .collect();
        let position = self.previous_action + reference_position;
        let (kp, kd) = policy.gains(&self.parameters);
        position_targets(position, kp, kd)
    }
}
