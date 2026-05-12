use std::{f32::consts::PI, time::Duration};

use approx::AbsDiffEq;
use booster::{JointsMotorState, MotorCommandParameters, MotorState};
use color_eyre::Result;
use coordinate_systems::{Ground, Robot};
use kinematics::joints::Joints;
use linear_algebra::{IntoFramed, Orientation2, Point2, Vector2, Vector3, vector};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{
    motion_command::{MotionCommand, OrientationMode},
    parameters::RLWalkingParameters,
    path::traits::{Length, PathProgress},
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
    pub joint_position_differences: Joints,
    pub joint_velocities: Joints,
    pub last_target_joint_positions: Joints,
}

pub enum WalkCommand {
    WalkWithVelocity {
        velocity: Vector2<Ground>,
        angular_velocity: f32,
    },
    Stand,
}

impl WalkCommand {
    pub fn from_motion_command(
        motion_command: &MotionCommand,
        parameters: &RLWalkingParameters,
    ) -> Self {
        match motion_command {
            MotionCommand::Walk {
                path,
                orientation_mode,
                target_orientation,
                distance_to_be_aligned,
                speed,
                ..
            } => {
                let forward = path.forward(Point2::origin());
                let distance_to_target = path.length();
                // let deceleration_factor =
                // (distance_to_target / parameters.deceleration_distance).clamp(0.0, 1.0);
                // let velocity = forward * *speed * deceleration_factor;

                let velocity = forward * *speed;

                let (walk_orientation, _tolerance): (Orientation2<Ground>, f32) =
                    match orientation_mode {
                        OrientationMode::Unspecified => todo!(),
                        OrientationMode::AlignWithPath => (Orientation2::from_vector(forward), 0.0),
                        OrientationMode::LookTowards {
                            direction,
                            tolerance,
                        } => (*direction, *tolerance),
                        OrientationMode::LookAt { target, tolerance } => (
                            Orientation2::from_vector(target - Point2::origin()),
                            *tolerance,
                        ),
                    };

                let target_alignment_importance = target_alignment_importance(
                    *distance_to_be_aligned,
                    parameters.hybrid_align_distance,
                    distance_to_target,
                );

                let orientation =
                    walk_orientation.slerp(*target_orientation, target_alignment_importance);

                let angular_velocity =
                    orientation.as_unit_vector().y() * parameters.max_alignment_rate;

                Self::WalkWithVelocity {
                    velocity,
                    angular_velocity,
                }
            }
            MotionCommand::WalkWithVelocity {
                velocity,
                angular_velocity,
                ..
            } => Self::WalkWithVelocity {
                velocity: *velocity,
                angular_velocity: *angular_velocity,
            },
            _ => Self::Stand,
        }
    }
}

// https://www.desmos.com/calculator/ng03egi9mp
fn target_alignment_importance(
    distance_to_be_aligned: f32,
    hybrid_align_distance: f32,
    distance_to_target: f32,
) -> f32 {
    if distance_to_target < distance_to_be_aligned {
        1.0
    } else if distance_to_target < distance_to_be_aligned + hybrid_align_distance {
        (1.0 + f32::cos(PI * (distance_to_target - distance_to_be_aligned) / hybrid_align_distance))
            * 0.5
    } else {
        0.0
    }
}

impl WalkingInferenceInputs {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        last_cycle_duration: Duration,
        walk_command: &WalkCommand,
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
            last_cycle_duration.as_secs_f32() * walking_parameters.control.decimation;

        let (linear_velocity_command, angular_velocity_command) = match walk_command {
            WalkCommand::WalkWithVelocity {
                velocity,
                angular_velocity,
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
            WalkCommand::Stand => (vector![0.0, 0.0], 0.0),
        };

        let stabilizing_interval_progress = last_gait_progress
            + walking_parameters.gait_frequency * last_cycle_duration.as_secs_f32();

        let is_step_finished = (stabilizing_interval_progress
            * walking_parameters.stabilizing_interval_compression_factor
            * PI)
            .sin()
            .abs_diff_eq(
                &0.0,
                walking_parameters.stabilizing_interval_completion_threshold,
            )
            || (stabilizing_interval_progress
                * walking_parameters.stabilizing_interval_compression_factor
                * PI)
                .cos()
                .abs_diff_eq(
                    &1.0,
                    walking_parameters.stabilizing_interval_completion_threshold,
                );

        let (gait_frequency, last_gait_progress) = if linear_velocity_command.norm() < 1e-5
            && angular_velocity_command.abs() < 1e-5
            && is_step_finished
        {
            (0.0, 0.0)
        } else {
            (walking_parameters.gait_frequency, last_gait_progress)
        };
        let gait_progress = last_gait_progress + gait_frequency * last_cycle_duration.as_secs_f32();

        let gait_process =
            nalgebra::Rotation2::new(2.0 * PI * gait_progress) * nalgebra::Vector2::x();

        let current_joint_position = current_serial_joints.positions();
        let current_joint_velocities = current_serial_joints.velocities();

        let joint_position_differences =
            current_joint_position - motor_command_parameters.default_positions;

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
        let normalized_joint_position_differences = joint_position_differences
            .into_iter()
            .map(|elem| elem * walking_parameters.normalization.joint_position)
            .collect();
        let normalized_joint_velocities = current_joint_velocities
            .into_iter()
            .map(|elem| elem * walking_parameters.normalization.joint_velocity)
            .collect();

        Ok(WalkingInferenceInputs {
            gravity,
            angular_velocity,
            linear_velocity_command,
            angular_velocity_command,
            gait_progress,
            gait_process,
            joint_position_differences: normalized_joint_position_differences,
            joint_velocities: normalized_joint_velocities,
            last_target_joint_positions,
        })
    }

    pub fn mjlab_walking_policy_observation_vector(&self) -> Vec<f32> {
        [
            self.angular_velocity.x(),
            self.angular_velocity.y(),
            self.angular_velocity.z(),
            self.gravity.x(),
            self.gravity.y(),
            self.gravity.z(),
        ]
        .into_iter()
        .chain(joints_as_array(self.joint_position_differences))
        .chain(joints_as_array(self.joint_velocities))
        .chain(joints_as_array(self.last_target_joint_positions))
        .chain([
            self.linear_velocity_command.x(),
            self.linear_velocity_command.y(),
            self.angular_velocity_command,
        ])
        .collect::<Vec<f32>>()
    }
}

fn joints_as_array(joints: Joints) -> [f32; 20] {
    // ALeft_Shoulder_Pitch,Left_Shoulder_Roll,Left_Elbow_Pitch,Left_Elbow_Yaw,
    // ARight_Shoulder_Pitch,Right_Shoulder_Roll,Right_Elbow_Pitch,Right_Elbow_Yaw,
    // Left_Hip_Pitch,Left_Hip_Roll,Left_Hip_Yaw,Left_Knee_Pitch,Left_Ankle_Pitch,Left_Ankle_Roll,
    // Right_Hip_Pitch,Right_Hip_Roll,Right_Hip_Yaw,Right_Knee_Pitch,Right_Ankle_Pitch,Right_Ankle_Roll
    [
        joints.left_arm.shoulder_pitch,
        joints.left_arm.shoulder_roll,
        joints.left_arm.elbow,
        joints.left_arm.shoulder_yaw,
        joints.right_arm.shoulder_pitch,
        joints.right_arm.shoulder_roll,
        joints.right_arm.elbow,
        joints.right_arm.shoulder_yaw,
        joints.left_leg.hip_pitch,
        joints.left_leg.hip_roll,
        joints.left_leg.hip_yaw,
        joints.left_leg.knee,
        joints.left_leg.ankle_up,
        joints.left_leg.ankle_down,
        joints.right_leg.hip_pitch,
        joints.right_leg.hip_roll,
        joints.right_leg.hip_yaw,
        joints.right_leg.knee,
        joints.right_leg.ankle_up,
        joints.right_leg.ankle_down,
    ]
}
