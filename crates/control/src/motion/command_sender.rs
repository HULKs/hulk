use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::AdditionalOutput;
use hardware::ActuatorInterface;
use serde::{Deserialize, Serialize};
use types::{led::Leds, motion_selection::MotionSafeExits, motor_command::MotorCommand};

#[derive(Deserialize, Serialize)]
pub struct CommandSender {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motor_command: Input<MotorCommand<f32>, "motor_command">,
    leds: Input<Leds, "leds">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
    last_executed_motor_command: CyclerState<MotorCommand<f32>, "last_executed_motor_command">,

    motion_safe_exits_output: AdditionalOutput<MotionSafeExits, "motion_safe_exits_output">,
    last_executed_motor_command_output:
        AdditionalOutput<MotorCommand<f32>, "last_executed_motor_command">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl CommandSender {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl ActuatorInterface>,
    ) -> Result<MainOutputs> {
        let motor_command = context.motor_command;

        context
            .hardware_interface
            .write_to_actuators(
                motor_command.positions,
                motor_command.stiffnesses,
                *context.leds,
            )
            .wrap_err("failed to write to actuators")?;

        context.last_executed_motor_command.positions = motor_command.positions;
        context.last_executed_motor_command.stiffnesses = motor_command.stiffnesses;

        context
            .last_executed_motor_command_output
            .fill_if_subscribed(|| *motor_command);
        context
            .motion_safe_exits_output
            .fill_if_subscribed(|| context.motion_safe_exits.clone());

        Ok(MainOutputs {})
    }
}
