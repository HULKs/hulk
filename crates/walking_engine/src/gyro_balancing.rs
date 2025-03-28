use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{
    joints::{body::LowerBodyJoints, leg::LegJoints},
    support_foot::Side,
};

use crate::{parameters, Context};

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
struct PidControl {
    integral: f32,
    previous_error: f32,
}

impl PidControl {
    pub fn calculate(&mut self, error: f32, dt: f32, parameters: &parameters::PidControl) -> f32 {
        self.integral += error * dt;
        self.integral = self
            .integral
            .clamp(-parameters.max_integral, parameters.max_integral);
        let derivative = (error - self.previous_error) / dt;
        self.previous_error = error;
        parameters.kp * error + parameters.ki * self.integral + parameters.kd * derivative
    }
}

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
pub struct UprightBalancing {
    balancing: LegJoints,
    roll_pid: PidControl,
    pitch_pid: PidControl,
}

impl UprightBalancing {
    pub fn tick(&mut self, context: &Context) {
        let gyro = context.gyro;
        let parameters = &context.parameters.upright_balancing;
        let dt = context.cycle_time.last_cycle_duration.as_secs_f32();

        let desired_torso_rotation = context.robot_to_walk.rotation();
        let current_orientation = context.robot_orientation;
        let orientation_error = current_orientation.inner * desired_torso_rotation.inner.inverse();
        let (roll_error, pitch_error, _) = orientation_error.euler_angles();

        let roll_pid_correction = self
            .roll_pid
            .calculate(roll_error, dt, &parameters.roll_pid);
        let pitch_pid_correction = self
            .pitch_pid
            .calculate(pitch_error, dt, &parameters.pitch_pid);
        let pid_correction = LegJoints {
            ankle_pitch: pitch_pid_correction,
            ankle_roll: roll_pid_correction,
            hip_pitch: 0.0,
            hip_roll: 0.0,
            hip_yaw_pitch: 0.0,
            knee_pitch: 0.0,
        };

        // Compute target balancing offsets for the support leg joints based on gyro feedback.
        let gyro_gains = &parameters.gyro_gains;
        let gyro_balancing = LegJoints {
            ankle_pitch: gyro_gains.ankle_pitch * gyro.y,
            ankle_roll: gyro_gains.ankle_roll * gyro.x,
            hip_pitch: gyro_gains.hip_pitch * gyro.y,
            hip_roll: gyro_gains.hip_roll * gyro.x,
            hip_yaw_pitch: 0.0,
            knee_pitch: gyro_gains.knee_pitch * gyro.y,
        };

        let support_balancing = pid_correction + gyro_balancing;

        let max_delta = parameters.max_delta;
        self.balancing =
            self.balancing + (support_balancing - self.balancing).clamp(-max_delta, max_delta);
    }
}

pub trait UprightBalancingExt {
    fn balance_using_imu(self, state: &UprightBalancing, support_side: Side) -> Self;
}

impl UprightBalancingExt for LowerBodyJoints {
    fn balance_using_imu(mut self, state: &UprightBalancing, support_side: Side) -> Self {
        let support_leg = match support_side {
            Side::Left => &mut self.left_leg,
            Side::Right => &mut self.right_leg,
        };
        *support_leg += state.balancing;
        self
    }
}
