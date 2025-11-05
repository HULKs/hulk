use std::{f32::consts::PI, time::SystemTime};

use color_eyre::{eyre::eyre, Result};
use coordinate_systems::{Ground, Robot};
use linear_algebra::{vector, Vector2, Vector3};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::{
    joints::{leg::LegJoints, Joints},
    motion_command::MotionCommand,
    parameters::{MotorCommandParameters, RLWalkingParameters},
};

/// # Model Input
///     Input dimension: (47)
///     - gravtiy 3
///     - angular velocity 3
///     - commands 3
///         - linear velocity x 1
///         - linear velocity y 1
///         - angular velocity yaw 1
///     - cos gait process 1
///     - sin gait process 1
///     - joint positions 12
///         ["left_hip_pitch", "left_hip_roll", "left_hip_yaw",
///         "left_knee_pitch", "left_ankle_pitch", "left_ankle_roll",
///         "right_hip_pitch", "right_hip_roll", "right_hip_yaw",
///         "right_knee_pitch", "right_ankle_pitch", "right_ankle_roll",]
///     - joint velocities 12
///     - actions 12
///         - last joint velocity targets (normalized)
///  # Model Output
///     Output Dimension: (12)
///     - Joint velocity targets
///
///
#[derive(
    Debug, Default, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct WalkingInferenceInputs {
    pub gravity: Vector3<Robot>,
    pub angular_velocity: Vector3<Robot>,
    pub linear_velocity_command: Vector2<Ground>,
    pub angular_velocity_command: f32,
    pub gait_progress_cos: f32,
    pub gait_progress_sin: f32,
    pub joint_position_differences: [f32; 12],
    pub joint_velocities: [f32; 12],
    pub last_target_joint_positions: [f32; 12],
}

impl WalkingInferenceInputs {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        now: SystemTime,
        roll_pitch_yaw: Vector3<Robot>,
        angular_velocity: Vector3<Robot>,
        motion_command: &MotionCommand,
        current_joint_positions: Joints,
        current_joint_velocities: Joints,
        last_target_left_joint_positions: LegJoints,
        last_target_right_joint_positions: LegJoints,
        last_linear_velocity_command: Vector2<Ground>,
        last_angular_velocity_command: f32,
        walking_parameters: RLWalkingParameters,
        motor_command_parameters: MotorCommandParameters,
    ) -> Result<Self> {
        let policy_interval = walking_parameters.control.dt * walking_parameters.control.decimation;

        let (linear_velocity_command, angular_velocity_command) = match motion_command {
            MotionCommand::WalkWithVelocity {
                velocity,
                angular_velocity,
                ..
            } => {
                let linear_velocity_command_difference = velocity - last_linear_velocity_command;
                let angular_velocity_command_difference =
                    angular_velocity - last_angular_velocity_command;
                (
                    last_linear_velocity_command
                        + vector!(
                            linear_velocity_command_difference
                                .x()
                                .clamp(-policy_interval, policy_interval,),
                            linear_velocity_command_difference
                                .y()
                                .clamp(-policy_interval, policy_interval,)
                        ),
                    last_angular_velocity_command
                        + angular_velocity_command_difference
                            .clamp(-policy_interval, policy_interval),
                )
            }
            _ => todo!(),
        };

        let gait_frequency = if linear_velocity_command.norm() < 1e-5 {
            0.0
        } else {
            walking_parameters.gait_frequency
        };
        let gait_process = (gait_frequency * now.elapsed()?.as_secs_f32()) % 1.0;

        let gait_progress_cos = f32::cos(2.0 * PI * gait_process);
        let gait_progress_sin = f32::sin(2.0 * PI * gait_process);

        let left_leg_position_difference =
            current_joint_positions.left_leg - motor_command_parameters.default_positions.left_leg;
        let right_leg_position_difference = current_joint_positions.right_leg
            - motor_command_parameters.default_positions.right_leg;
        let joint_position_differences = left_leg_position_difference
            .into_iter()
            .chain(right_leg_position_difference.into_iter())
            .collect::<Vec<f32>>()
            .try_into()
            .map_err(|v: Vec<f32>| eyre!("expected 12 joint positions but got {}", v.len()))?;

        let joint_velocities = current_joint_velocities
            .left_leg
            .into_iter()
            .chain(current_joint_velocities.right_leg.into_iter())
            .collect::<Vec<f32>>()
            .try_into()
            .map_err(|v: Vec<f32>| eyre!("expected 12 joint velocities but got {}", v.len()))?;

        let last_target_joint_positions = last_target_left_joint_positions
            .into_iter()
            .chain(last_target_right_joint_positions.into_iter())
            .collect::<Vec<f32>>()
            .try_into()
            .map_err(|v: Vec<f32>| {
                eyre!(
                    "expected 12 last target joint positions but got {}",
                    v.len()
                )
            })?;

        let rotation = nalgebra::Rotation3::from_euler_angles(
            roll_pitch_yaw.x(),
            roll_pitch_yaw.y(),
            roll_pitch_yaw.z(),
        );
        let gravity = rotation.transform_vector(&-nalgebra::Vector3::z_axis());

        Ok(WalkingInferenceInputs {
            gravity: vector!(gravity.x, gravity.y, gravity.z),
            angular_velocity,
            linear_velocity_command,
            angular_velocity_command,
            gait_progress_cos,
            gait_progress_sin,
            joint_position_differences,
            joint_velocities,
            last_target_joint_positions,
        })
    }

    pub fn normalize(mut self, parameters: RLWalkingParameters) -> Self {
        let parameters = &parameters.normalization;
        self.gravity *= parameters.linear_velocity;
        self.linear_velocity_command *= parameters.linear_velocity;
        self.angular_velocity_command *= parameters.angular_velocity;
        self.joint_position_differences = self
            .joint_position_differences
            .map(|elem| elem * parameters.joint_position);
        self.joint_velocities = self
            .joint_velocities
            .map(|elem| elem * parameters.joint_velocity);
        self
    }

    pub fn as_vec(&self) -> Vec<f32> {
        [
            self.gravity.x(),
            self.gravity.y(),
            self.gravity.z(),
            self.angular_velocity.x(),
            self.angular_velocity.y(),
            self.angular_velocity.z(),
            self.linear_velocity_command.x(),
            self.linear_velocity_command.y(),
            self.angular_velocity_command,
            self.gait_progress_cos,
            self.gait_progress_sin,
        ]
        .iter()
        .chain(self.joint_position_differences.iter())
        .chain(self.joint_velocities.iter())
        .chain(self.last_target_joint_positions.iter())
        .copied()
        .collect::<Vec<f32>>()
    }
}

pub fn rotate_vector_inverse(
    linear_acceleration: Vector3<Robot>,
    rotation_vector: Vector3<Robot>,
) -> Vector3<Robot> {
    // todo!("Implement this properly using linear_algebra");

    let cos_roll = linear_acceleration.x().cos();
    let sin_roll = linear_acceleration.x().sin();
    let cos_pitch = linear_acceleration.y().cos();
    let sin_pitch = linear_acceleration.y().sin();
    let cos_yaw = linear_acceleration.z().cos();
    let sin_yaw = linear_acceleration.z().sin();

    let r_x = nalgebra::Matrix3::new(
        1.0, 0.0, 0.0, 0.0, cos_roll, -sin_roll, 0.0, sin_roll, cos_roll,
    );

    let r_y = nalgebra::Matrix3::new(
        cos_pitch, 0.0, sin_pitch, 0.0, 1.0, 0.0, -sin_pitch, 0.0, cos_pitch,
    );

    let r_z = nalgebra::Matrix3::new(cos_yaw, -sin_yaw, 0.0, sin_yaw, cos_yaw, 0.0, 0.0, 0.0, 1.0);

    let rotation_matrix = r_z * r_y * r_x;
    let vector = rotation_matrix.transpose() * rotation_vector.inner;
    vector!(vector[0], vector[1], vector[2])
}
