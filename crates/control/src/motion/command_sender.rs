use booster::LowCommand;
use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use hardware::{LowCommandInterface, TimeInterface};
use serde::{Deserialize, Serialize};
use types::{joints::Joints, parameters::MotorCommandParameters};

#[derive(Deserialize, Serialize)]
pub struct CommandSender {
    time_index: f32,
    motor_index: usize,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    target_joint_positions: Input<Joints, "target_joint_positions">,

    walk_motor_command_parameters: Parameter<MotorCommandParameters, "common_motor_command">,
    prepare_motor_command_parameters: Parameter<MotorCommandParameters, "prepare_motor_command">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl CommandSender {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            time_index: 0.0,
            motor_index: 0,
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl LowCommandInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        let low_command = LowCommand::new(
            context.target_joint_positions,
            context.walk_motor_command_parameters,
        );

        context
            .hardware_interface
            .write_low_command(low_command)
            .wrap_err("failed to write to actuators")?;

        Ok(MainOutputs {})
    }
}
