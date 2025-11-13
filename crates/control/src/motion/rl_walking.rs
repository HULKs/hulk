use std::{
    f32::consts::PI,
    time::{SystemTime, UNIX_EPOCH},
};

use booster::{ImuState, JointsMotorState, MotorState};
use color_eyre::{eyre::ContextCompat, Result};
use context_attribute::context;
use coordinate_systems::{Ground, Robot};
use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use hardware::{PathsInterface, TimeInterface};
use itertools::Itertools;
use linear_algebra::{vector, IntoFramed, Vector2, Vector3};
use ndarray::{Array1, Axis};
use ort::{
    inputs,
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
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
    prepare_motor_command_parameters: Parameter<MotorCommandParameters, "prepare_motor_command">,

    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    walking_parameters: Parameter<RLWalkingParameters, "rl_walking">,
    common_motor_command_parameters: Parameter<MotorCommandParameters, "common_motor_command">,

    // walking_inference_inputs: AdditionalOutput<WalkingInferenceInputs, "walking_inference_inputs">,
    imu_state: Input<ImuState, "imu_state">,
    serial_motor_states: Input<Joints<MotorState>, "serial_motor_states">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub target_joint_positions: MainOutput<Joints>,
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
            last_target_left_joint_positions: context
                .prepare_motor_command_parameters
                .default_positions
                .left_leg,
            last_target_right_joint_positions: context
                .prepare_motor_command_parameters
                .default_positions
                .right_leg,
            last_linear_velocity_command: vector![0.0, 0.0],
            last_angular_velocity_command: 0.0,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext<impl TimeInterface>) -> Result<MainOutputs> {
        let walking_inference_inputs = WalkingInferenceInputs::try_new(
            context.hardware_interface.get_now(),
            context.imu_state.roll_pitch_yaw,
            context.imu_state.angular_velocity,
            &MotionCommand::WalkWithVelocity {
                velocity: vector!(
                    context.walking_parameters.walk_command[0],
                    context.walking_parameters.walk_command[1]
                ),
                angular_velocity: context.walking_parameters.walk_command[2],
                head: HeadMotion::Center,
            },
            *context.serial_motor_states,
            self.last_target_left_joint_positions,
            self.last_target_right_joint_positions,
            self.last_linear_velocity_command,
            self.last_angular_velocity_command,
            context.walking_parameters.clone(),
            context.common_motor_command_parameters.clone(),
        )?
        .normalize(context.walking_parameters.clone());

        // context
        //     .walking_inference_inputs
        //     .fill_if_subscribed(|| walking_inference_inputs.clone());

        self.last_linear_velocity_command = walking_inference_inputs.linear_velocity_command;
        self.last_angular_velocity_command = walking_inference_inputs.angular_velocity_command;

        let inputs: Array1<f32> = walking_inference_inputs.as_vec().into();

        assert!(inputs.len() == context.walking_parameters.number_of_observations);
        let inputs_tensor = Tensor::from_array(inputs.insert_axis(Axis(0)))?;

        let outputs = self.session.run(inputs![inputs_tensor])?;
        let predictions = outputs["15"].try_extract_array::<f32>()?.squeeze();

        predictions.clamp(
            -context.walking_parameters.normalization.clip_actions,
            context.walking_parameters.normalization.clip_actions,
        );

        assert!(predictions.len() == context.walking_parameters.number_of_actions);
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

        self.last_target_left_joint_positions = left_leg_predictions;
        self.last_target_right_joint_positions = right_leg_predictions;

        let mut target_joint_positions = context.common_motor_command_parameters.default_positions;
        target_joint_positions.left_leg +=
            left_leg_predictions * context.walking_parameters.control.action_scale;
        target_joint_positions.right_leg +=
            right_leg_predictions * context.walking_parameters.control.action_scale;

        Ok(MainOutputs {
            target_joint_positions: target_joint_positions.into(),
        })
    }
}

#[derive(
    Debug, Default, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct WalkingInferenceInputs {
    pub gravity: Vector3<Robot>,
    pub angular_velocity: Vector3<Robot>,
    pub linear_velocity_command: Vector2<Ground>,
    pub angular_velocity_command: f32,
    pub gait_process: nalgebra::Vector2<f32>,
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
        current_serial_joints: Joints<MotorState>,
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
        let gait_progress = (gait_frequency * now.duration_since(UNIX_EPOCH)?.as_secs_f32()) % 1.0;

        let is_walking = gait_frequency > 1.0e-8;
        let gait_process = if is_walking {
            nalgebra::Rotation2::new(2.0 * PI * gait_progress) * nalgebra::Vector2::x()
        } else {
            Default::default()
        };

        let current_joint_position = current_serial_joints.joint_positions();
        let current_joint_velocities = current_serial_joints.joint_velocities();

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

        let last_target_joint_positions = last_target_left_joint_positions
            .into_iter()
            .chain(last_target_right_joint_positions.into_iter())
            .collect_array()
            .wrap_err("expected 12 last targetjoint positions")?;

        let rotation = nalgebra::Rotation3::from_euler_angles(
            roll_pitch_yaw.x(),
            roll_pitch_yaw.y(),
            roll_pitch_yaw.z(),
        );
        let gravity = rotation
            .inverse()
            .transform_vector(&-nalgebra::Vector3::z_axis())
            .framed();

        Ok(WalkingInferenceInputs {
            gravity,
            angular_velocity,
            linear_velocity_command,
            angular_velocity_command,
            gait_process,
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
