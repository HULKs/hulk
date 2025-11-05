use booster::ImuState;
use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use hardware::{PathsInterface, TimeInterface};
use linear_algebra::{vector, Vector2};
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
    walking_inference_inputs::WalkingInferenceInputs,
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

    walking_inference_inputs: AdditionalOutput<WalkingInferenceInputs, "walking_inference_inputs">,

    imu_state: Input<ImuState, "low_state.imu_state">,
    joint_positions: Input<Joints, "joint_positions">,
    joint_velocities: Input<Joints, "joint_velocities">,

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
            last_linear_velocity_command: vector!(0.0, 0.0),
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
            *context.joint_positions,
            *context.joint_velocities,
            self.last_target_left_joint_positions,
            self.last_target_right_joint_positions,
            self.last_linear_velocity_command,
            self.last_angular_velocity_command,
            context.walking_parameters.clone(),
            context.common_motor_command_parameters.clone(),
        )?
        .normalize(context.walking_parameters.clone());

        context
            .walking_inference_inputs
            .fill_if_subscribed(|| walking_inference_inputs.clone());

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
