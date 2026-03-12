use booster::{CommandType, LowCommand, MotorCommandParameters};
use color_eyre::{Result, eyre::WrapErr};
use context_attribute::context;
use framework::AdditionalOutput;
use hardware::{LowCommandInterface, MotionRuntimeInteface};
use kinematics::joints::Joints;
use serde::{Deserialize, Serialize};
use types::motion_runtime::MotionRuntime;

#[derive(Deserialize, Serialize)]
pub struct CommandSender {
    time_index: f32,
    motor_index: usize,
    filtered_target_joint_positions: Joints,
}

#[context]
pub struct CreationContext {
    prepare_motor_command_parameters: Parameter<MotorCommandParameters, "prepare_motor_command">,
}

#[context]
pub struct CycleContext {
    low_command: AdditionalOutput<LowCommand, "low_command">,

    walk_motor_command_parameters: Parameter<MotorCommandParameters, "common_motor_command">,
    _prepare_motor_command_parameters: Parameter<MotorCommandParameters, "prepare_motor_command">,

    hardware_interface: HardwareInterface,
    collected_target_joint_positions: Input<Joints<f32>, "collected_target_joint_positions">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl CommandSender {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            time_index: 0.0,
            motor_index: 0,
            filtered_target_joint_positions: context
                .prepare_motor_command_parameters
                .default_positions,
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl LowCommandInterface + MotionRuntimeInteface>,
    ) -> Result<MainOutputs> {
        if context.hardware_interface.get_motion_runtime_type()? != MotionRuntime::Hulk {
            return Ok(MainOutputs {});
        }

        let walk_low_command = LowCommand::new(
            context.collected_target_joint_positions,
            context.walk_motor_command_parameters,
            CommandType::Serial,
        );

        context
            .hardware_interface
            .write_low_command(walk_low_command.clone())
            .wrap_err("failed to write to actuators")?;

        context
            .low_command
            .fill_if_subscribed(|| walk_low_command.clone());

        Ok(MainOutputs {})
    }
}
