use std::f32::consts::PI;

use booster::{JointsMotorState, MotorState};
use color_eyre::{eyre::ContextCompat, Result};
use coordinate_systems::{Ground, Robot};
use itertools::Itertools;
use linear_algebra::{vector, IntoFramed, Vector2, Vector3};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::Joints,
    motion_command::MotionCommand,
    parameters::{MotorCommandParameters, RLWalkingParameters},
};

#[derive(
    Debug, Default, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct WalkingInferenceInputs {
    pub gravity: Vector3<Robot>,
    pub angular_velocity: Vector3<Robot>,
    pub linear_velocity_command: Vector2<Ground>,
    pub angular_velocity_command: f32,
    pub gait_progress: f32,
    pub gait_process: nalgebra::Vector2<f32>,
    pub joint_position_differences: [f32; 12],
    pub joint_velocities: [f32; 12],
    pub last_target_joint_positions: [f32; 12],
}

impl WalkingInferenceInputs {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        cycle_time: CycleTime,
        motion_command: &MotionCommand,
        roll_pitch_yaw: Vector3<Robot>,
        angular_velocity: Vector3<Robot>,
        current_serial_joints: Joints<MotorState>,
        last_linear_velocity_command: Vector2<Ground>,
        last_angular_velocity_command: f32,
        last_gait_progress: f32,
        last_target_joint_positions: Joints,
        walking_parameters: &RLWalkingParameters,
        motor_command_parameters: &MotorCommandParameters,
    ) -> Result<Self> {
        let policy_interval =
            cycle_time.last_cycle_duration.as_secs_f32() * walking_parameters.control.decimation;

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
                        + vector![
                            linear_velocity_command_difference
                                .x()
                                .clamp(-policy_interval, policy_interval,),
                            linear_velocity_command_difference
                                .y()
                                .clamp(-policy_interval, policy_interval,)
                        ],
                    last_angular_velocity_command
                        + angular_velocity_command_difference
                            .clamp(-policy_interval, policy_interval),
                )
            }
            MotionCommand::Stand { .. } | MotionCommand::Unstiff => (vector![0.0, 0.0], 0.0),
            _ => todo!(),
        };

        let (gait_frequency, last_gait_progress) =
            if linear_velocity_command.norm() < 1e-5 && angular_velocity_command.abs() < 1e-5 {
                (0.0, 0.0)
            } else {
                (walking_parameters.gait_frequency, last_gait_progress)
            };
        let gait_progress =
            last_gait_progress + gait_frequency * cycle_time.last_cycle_duration.as_secs_f32();

        let gait_process =
            nalgebra::Rotation2::new(2.0 * PI * gait_progress) * nalgebra::Vector2::x();

        let current_joint_position = current_serial_joints.positions();
        let current_joint_velocities = current_serial_joints.velocities();

        let left_leg_position_difference =
            current_joint_position.left_leg - motor_command_parameters.default_positions.left_leg;
        let right_leg_position_difference =
            current_joint_position.right_leg - motor_command_parameters.default_positions.right_leg;
        let joint_position_differences = left_leg_position_difference
            .into_iter()
            .chain(right_leg_position_difference.into_iter())
            .collect_array()
            .wrap_err("expected 12 joint position differences")?;

        let joint_velocities = current_joint_velocities
            .left_leg
            .into_iter()
            .chain(current_joint_velocities.right_leg.into_iter())
            .collect_array()
            .wrap_err("expected 12 joint velocities")?;

        let last_target_joint_positions = last_target_joint_positions
            .left_leg
            .into_iter()
            .chain(last_target_joint_positions.right_leg.into_iter())
            .collect_array()
            .wrap_err("expected 12 last target joint positions")?;

        let rotation = nalgebra::Rotation3::from_euler_angles(
            roll_pitch_yaw.x(),
            roll_pitch_yaw.y(),
            roll_pitch_yaw.z(),
        );
        let gravity = rotation
            .inverse()
            .transform_vector(&-nalgebra::Vector3::z_axis())
            .framed()
            * walking_parameters.normalization.linear_velocity;

        let linear_velocity_command =
            linear_velocity_command * walking_parameters.normalization.linear_velocity;
        let angular_velocity_command =
            angular_velocity_command * walking_parameters.normalization.angular_velocity;
        let joint_position_differences = joint_position_differences
            .map(|elem| elem * walking_parameters.normalization.joint_position);
        let joint_velocities =
            joint_velocities.map(|elem| elem * walking_parameters.normalization.joint_velocity);

        Ok(WalkingInferenceInputs {
            gravity,
            angular_velocity,
            linear_velocity_command,
            angular_velocity_command,
            gait_progress,
            gait_process,
            joint_position_differences,
            joint_velocities,
            last_target_joint_positions,
        })
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
            self.gait_process.x,
            self.gait_process.y,
        ]
        .iter()
        .chain(self.joint_position_differences.iter())
        .chain(self.joint_velocities.iter())
        .chain(self.last_target_joint_positions.iter())
        .copied()
        .collect::<Vec<f32>>()
    }
}
