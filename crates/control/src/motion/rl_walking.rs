use std::f32::consts::PI;

use booster::ImuState;
use color_eyre::{eyre::eyre, Result};
use context_attribute::context;
use coordinate_systems::{Ground, Robot};
use framework::{deserialize_not_implemented, MainOutput};
use hardware::{PathsInterface, TimeInterface};
use linear_algebra::{vector, Vector2, Vector3};
use ndarray::{Array1, Axis};
use ort::{
    inputs,
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use serde::{Deserialize, Serialize};
use types::{
    joints::{leg::LegJoints, Joints},
    motion_command::{HeadMotion, MotionCommand},
    parameters::{MotorCommandParameters, RLWalkingParameters},
};

#[derive(Deserialize, Serialize)]
pub struct RLWalking {
    #[serde(skip, default = "deserialize_not_implemented")]
    session: Session,
    last_target_left_joint_positions: LegJoints,
    last_target_right_joint_positions: LegJoints,
    last_linear_velocity_command: Vector2<Ground>,
    last_angular_velocity_command: f32,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    parameters: Parameter<RLWalkingParameters, "rl_walking">,
    default_joint_positions: Parameter<Joints, "common_motor_command.default_positions">,
    common_motor_command:
        Parameter<MotorCommandParameters, "common_motor_command.derivative_coefficients">,
    prepare_motor_command:
        Parameter<MotorCommandParameters, "common_motor_command.proportional_coefficients">,

    imu_state: Input<ImuState, "low_state.imu_state">,
    joint_positions: Input<Joints, "joint_positions">,
    joint_velocities: Input<Joints, "joint_velocities">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub target_joint_velocities: MainOutput<Joints>,
}

impl RLWalking {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;
        let neural_network_path = neural_network_folder.join("T1.onnx");

        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(neural_network_path)?;
        Ok(Self {
            session,
            last_target_left_joint_positions: Default::default(),
            last_target_right_joint_positions: Default::default(),
            last_linear_velocity_command: vector!(0.0, 0.0),
            last_angular_velocity_command: 0.0,
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl TimeInterface>) -> Result<MainOutputs> {
        let walking_inference_inputs = WalkingInferenceInputs::try_new(
            context.imu_state,
            &MotionCommand::WalkWithVelocity {
                velocity: vector!(1.0, 0.0),
                angular_velocity: 0.0,
                head: HeadMotion::Center,
            },
            self.last_target_left_joint_positions,
            self.last_target_right_joint_positions,
            self.last_linear_velocity_command,
            self.last_angular_velocity_command,
            &context,
        )?
        .normalize(&context);

        self.last_linear_velocity_command = walking_inference_inputs.linear_velocity_command;
        self.last_angular_velocity_command = walking_inference_inputs.angular_velocity_command;

        let inputs = walking_inference_inputs.as_array();

        assert!(inputs.len() == context.parameters.number_of_observations);
        let inputs_tensor = Tensor::from_array(inputs.insert_axis(Axis(0)))?;

        let outputs = self.session.run(inputs![inputs_tensor])?;
        let predictions = outputs["15"].try_extract_array::<f32>()?.squeeze();

        predictions.clamp(
            -context.parameters.normalization.clip_actions,
            context.parameters.normalization.clip_actions,
        );

        assert!(predictions.len() == context.parameters.number_of_actions);
        let left_leg_predictions = LegJoints {
            hip_pitch: predictions[0],
            hip_roll: predictions[1],
            hip_yaw: predictions[2],
            knee: predictions[3],
            ankle_up: predictions[4],
            ankle_down: predictions[5],
        };
        let right_leg_predictions = LegJoints {
            hip_pitch: predictions[6],
            hip_roll: predictions[7],
            hip_yaw: predictions[8],
            knee: predictions[9],
            ankle_up: predictions[10],
            ankle_down: predictions[11],
        };

        let mut target_joint_positions = *context.joint_positions;
        target_joint_positions.left_leg = context.joint_positions.left_leg
            + (left_leg_predictions * context.parameters.control.action_scale);
        target_joint_positions.right_leg = context.joint_positions.right_leg
            + (right_leg_predictions * context.parameters.control.action_scale);
        Ok(MainOutputs {
            target_joint_velocities: target_joint_positions.into(),
        })
    }
}

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
pub struct WalkingInferenceInputs {
    pub linear_acceleration: Vector3<Robot>,
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
    fn try_new(
        imu_state: &ImuState,
        motion_command: &MotionCommand,
        last_target_left_joint_positions: LegJoints,
        last_target_right_joint_positions: LegJoints,
        last_linear_velocity_command: Vector2<Ground>,
        last_angular_velocity_command: f32,
        context: &CycleContext<impl TimeInterface>,
    ) -> Result<Self> {
        let policy_interval = context.parameters.control.dt * context.parameters.control.decimation;

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
            context.parameters.gait_frequency
        };
        let now = context.hardware_interface.get_now();
        let gait_process = (gait_frequency * now.elapsed()?.as_secs_f32()) % 1.0;

        let gait_progress_cos = f32::cos(2.0 * PI * gait_process);
        let gait_progress_sin = f32::sin(2.0 * PI * gait_process);

        let left_leg_position_difference =
            context.joint_positions.left_leg - context.default_joint_positions.left_leg;
        let right_leg_position_difference =
            context.joint_positions.right_leg - context.default_joint_positions.right_leg;
        let joint_position_differences = left_leg_position_difference
            .into_iter()
            .chain(right_leg_position_difference.into_iter())
            .collect::<Vec<f32>>()
            .try_into()
            .map_err(|v: Vec<f32>| eyre!("expected 12 joint positions but got {}", v.len()))?;

        let joint_velocities = context
            .joint_velocities
            .left_leg
            .into_iter()
            .chain(context.joint_velocities.right_leg.into_iter())
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
                    "expected 12 last target joint velocities but got {}",
                    v.len()
                )
            })?;

        Ok(WalkingInferenceInputs {
            linear_acceleration: imu_state.linear_acceleration,
            angular_velocity: imu_state.angular_velocity,
            linear_velocity_command,
            angular_velocity_command,
            gait_progress_cos,
            gait_progress_sin,
            joint_position_differences,
            joint_velocities,
            last_target_joint_positions,
        })
    }

    fn normalize(mut self, context: &CycleContext<impl TimeInterface>) -> Self {
        let parameters = &context.parameters.normalization;
        self.linear_acceleration *= parameters.linear_velocity;
        self.linear_velocity_command *= parameters.linear_velocity;
        self.angular_velocity_command *= parameters.angular_velocity;
        self
    }

    fn as_array(&self) -> Array1<f32> {
        [
            self.linear_acceleration.x(),
            self.linear_acceleration.y(),
            self.linear_acceleration.z(),
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
        .into()
    }
}
