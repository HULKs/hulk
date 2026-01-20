use booster::{ImuState, MotorState};
use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use hardware::PathsInterface;
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::Joints,
    motion_command::MotionCommand,
    parameters::{MotorCommandParameters, RLWalkingParameters},
};
use walking_inference::{inference::WalkingInference, inputs::WalkingInferenceInputs};

#[derive(Deserialize, Serialize)]
pub struct RLWalking {
    walking_inference: WalkingInference,
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
    motor_states: Input<Joints<MotorState>, "motor_states">,
    motion_command: Input<MotionCommand, "selected_motion_command">,
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

        let walking_inference = WalkingInference::new(
            &neural_network_folder,
            context.prepare_motor_command_parameters,
        )?;

        Ok(Self {
            walking_inference,
            smoothed_target_joint_positions: context
                .prepare_motor_command_parameters
                .default_positions,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let (walking_inference_inputs, inference_output_positions) =
            self.walking_inference.do_inference(
                *context.cycle_time,
                context.motion_command,
                context.imu_state,
                *context.motor_states,
                context.walking_parameters,
                context.common_motor_command_parameters,
            )?;

        context
            .walking_inference_inputs
            .fill_if_subscribed(|| walking_inference_inputs.clone());

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
