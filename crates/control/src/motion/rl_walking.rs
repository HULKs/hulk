use booster::{ImuState, MotorState};
use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::{AdditionalOutput, MainOutput};
use hardware::PathsInterface;
use linear_algebra::{vector, Vector2};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::Joints,
    motion_command::{HeadMotion, MotionCommand},
    parameters::{MotorCommandParameters, RLWalkingParameters},
};
use walking_inference::{inference::WalkingInference, inputs::WalkingInferenceInputs};

#[derive(Deserialize, Serialize)]
pub struct RLWalking {
    walking_inference: WalkingInference,
    last_linear_velocity_command: Vector2<Ground>,
    last_angular_velocity_command: f32,
    last_gait_progress: f32,
    last_target_joint_positions: Joints,
    smoothed_target_joint_positions: Joints,
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

    imu_state: Input<ImuState, "imu_state">,
    serial_motor_states: Input<Joints<MotorState>, "serial_motor_states">,
    cycle_time: Input<CycleTime, "cycle_time">,
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

        let walking_inference = WalkingInference::new(&neural_network_folder)?;

        Ok(Self {
            walking_inference,
            last_linear_velocity_command: vector![0.0, 0.0],
            last_angular_velocity_command: 0.0,
            last_gait_progress: 0.0,
            last_target_joint_positions: context.prepare_motor_command_parameters.default_positions,
            smoothed_target_joint_positions: context
                .prepare_motor_command_parameters
                .default_positions,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let motion_command = &MotionCommand::WalkWithVelocity {
            velocity: vector!(
                context.walking_parameters.walk_command[0],
                context.walking_parameters.walk_command[1]
            ),
            angular_velocity: context.walking_parameters.walk_command[2],
            head: HeadMotion::Center,
        };

        let walking_inference_inputs = WalkingInferenceInputs::try_new(
            *context.cycle_time,
            context.imu_state.roll_pitch_yaw,
            context.imu_state.angular_velocity,
            motion_command,
            *context.serial_motor_states,
            self.last_linear_velocity_command,
            self.last_angular_velocity_command,
            self.last_gait_progress,
            self.last_target_joint_positions,
            context.walking_parameters,
            context.common_motor_command_parameters,
        )?;

        self.last_linear_velocity_command = walking_inference_inputs.linear_velocity_command;
        self.last_angular_velocity_command = walking_inference_inputs.angular_velocity_command;
        self.last_gait_progress = walking_inference_inputs.gait_progress;

        context
            .walking_inference_inputs
            .fill_if_subscribed(|| walking_inference_inputs.clone());

        let inference_output_positions = self
            .walking_inference
            .do_inference(walking_inference_inputs, context.walking_parameters)?;

        self.last_target_joint_positions = inference_output_positions;

        let target_joint_positions = context.common_motor_command_parameters.default_positions
            + inference_output_positions * context.walking_parameters.control.action_scale;

        self.smoothed_target_joint_positions = self.smoothed_target_joint_positions
            * context.walking_parameters.joint_position_smoothing_factor
            + target_joint_positions
                * (1.0 - context.walking_parameters.joint_position_smoothing_factor);

        Ok(MainOutputs {
            target_joint_positions: self.smoothed_target_joint_positions.into(),
        })
    }
}
