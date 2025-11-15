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
    gait_progress: f32,
    last_target_joint_positions: Joints,
    last_linear_velocity_command: Vector2<Ground>,
    last_angular_velocity_command: f32,
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
            gait_progress: 0.0,
            last_target_joint_positions: context.prepare_motor_command_parameters.default_positions,
            last_linear_velocity_command: vector![0.0, 0.0],
            last_angular_velocity_command: 0.0,
            smoothed_target_joint_positions: context
                .prepare_motor_command_parameters
                .default_positions,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let policy_interval =
            context.walking_parameters.control.dt * context.walking_parameters.control.decimation;

        let motion_command = &MotionCommand::WalkWithVelocity {
            velocity: vector!(
                context.walking_parameters.walk_command[0],
                context.walking_parameters.walk_command[1]
            ),
            angular_velocity: context.walking_parameters.walk_command[2],
            head: HeadMotion::Center,
        };

        let (linear_velocity_command, angular_velocity_command) = match motion_command {
            MotionCommand::WalkWithVelocity {
                velocity,
                angular_velocity,
                ..
            } => {
                let linear_velocity_command_difference =
                    velocity - self.last_linear_velocity_command;
                let angular_velocity_command_difference =
                    angular_velocity - self.last_angular_velocity_command;
                (
                    self.last_linear_velocity_command
                        + vector!(
                            linear_velocity_command_difference
                                .x()
                                .clamp(-policy_interval, policy_interval,),
                            linear_velocity_command_difference
                                .y()
                                .clamp(-policy_interval, policy_interval,)
                        ),
                    self.last_angular_velocity_command
                        + angular_velocity_command_difference
                            .clamp(-policy_interval, policy_interval),
                )
            }
            _ => todo!(),
        };

        let gait_frequency =
            if linear_velocity_command.norm() < 1e-5 && angular_velocity_command.abs() < 1e-5 {
                0.0
            } else {
                context.walking_parameters.gait_frequency
            };
        self.gait_progress += gait_frequency * context.cycle_time.last_cycle_duration.as_secs_f32();

        let walking_inference_inputs = WalkingInferenceInputs::try_new(
            context.imu_state.roll_pitch_yaw,
            context.imu_state.angular_velocity,
            linear_velocity_command,
            angular_velocity_command,
            self.gait_progress,
            *context.serial_motor_states,
            self.last_target_joint_positions,
            context.common_motor_command_parameters.clone(),
        )?
        .normalize(context.walking_parameters.clone());

        self.last_linear_velocity_command = walking_inference_inputs.linear_velocity_command;
        self.last_angular_velocity_command = walking_inference_inputs.angular_velocity_command;

        context
            .walking_inference_inputs
            .fill_if_subscribed(|| walking_inference_inputs.clone());

        let inference_output_positions = self
            .walking_inference
            .do_inference(walking_inference_inputs, context.walking_parameters)?;

        self.last_target_joint_positions = inference_output_positions;

        let target_joint_positions = context.common_motor_command_parameters.default_positions
            + inference_output_positions * context.walking_parameters.control.action_scale;

        self.smoothed_target_joint_positions =
            self.smoothed_target_joint_positions * 0.8 + target_joint_positions * 0.2;

        Ok(MainOutputs {
            target_joint_positions: self.smoothed_target_joint_positions.into(),
        })
    }
}
