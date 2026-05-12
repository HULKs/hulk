use std::time::{Duration, SystemTime};

use booster::{ImuState, MotorCommandParameters, MotorState};
use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::PathsInterface;
use kinematics::joints::Joints;
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime, motion_command::MotionCommand, parameters::RLWalkingParameters,
};
use walking_inference::{inference::WalkingInference, inputs::WalkCommand};

#[derive(Deserialize, Serialize)]
pub struct RLWalking {
    walking_inference: WalkingInference,
    smoothed_target_joint_positions: Joints,
    next_inference_time: SystemTime,
}

#[context]
pub struct CreationContext {
    prepare_motor_command_parameters: Parameter<MotorCommandParameters, "prepare_motor_command">,
    walking_parameters: Parameter<RLWalkingParameters, "rl_walking">,

    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    walking_parameters: Parameter<RLWalkingParameters, "rl_walking">,
    common_motor_command_parameters: Parameter<MotorCommandParameters, "common_motor_command">,

    imu_state: Input<ImuState, "imu_state">,
    serial_motor_states: Input<Joints<MotorState>, "serial_motor_states">,
    motion_command: Input<MotionCommand, "WorldState", "motion_command">,
    cycle_time: Input<CycleTime, "cycle_time">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub walking_target_joint_positions: MainOutput<Option<Joints>>,
}

impl RLWalking {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let walking_inference = WalkingInference::new(
            &neural_network_folder,
            context.walking_parameters.observation_history_length,
        )?;

        Ok(Self {
            walking_inference,
            smoothed_target_joint_positions: context
                .prepare_motor_command_parameters
                .default_positions,
            next_inference_time: SystemTime::UNIX_EPOCH,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        if context.cycle_time.start_time < self.next_inference_time {
            return Ok(MainOutputs {
                walking_target_joint_positions: None.into(),
            });
        }

        self.next_inference_time = context.cycle_time.start_time
            + Duration::from_secs_f32(
                context.walking_parameters.control.dt
                    * context.walking_parameters.control.decimation,
            );

        let _walk_command =
            WalkCommand::from_motion_command(context.motion_command, context.walking_parameters);

        let inference_output_positions = self.walking_inference.do_inference(
            context.cycle_time.last_cycle_duration,
            &WalkCommand::Stand, // Nur standing policy!!! Nicht ändern, @BenSampaolo
            context.imu_state,
            *context.serial_motor_states,
            context.walking_parameters,
            context.common_motor_command_parameters,
        )?;

        let walking_target_joint_positions = (inference_output_positions
            * context.walking_parameters.control.action_scale)
            + context.common_motor_command_parameters.default_positions;

        self.smoothed_target_joint_positions = self.smoothed_target_joint_positions
            * context.walking_parameters.joint_position_smoothing_factor
            + walking_target_joint_positions
                * (1.0 - context.walking_parameters.joint_position_smoothing_factor);

        Ok(MainOutputs {
            walking_target_joint_positions: Some(self.smoothed_target_joint_positions).into(),
        })
    }
}
