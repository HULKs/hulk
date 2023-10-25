use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::AdditionalOutput;
use hardware::ActuatorInterface;
use serde::{Deserialize, Serialize};
use types::{led::Leds, motion_selection::MotionSafeExits, motor_commands::MotorCommands};

#[derive(Deserialize, Serialize)]
pub struct CommandSender {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motor_commands: Input<MotorCommands<f32>, "motor_command">,
    leds: Input<Leds, "leds">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
    last_executed_motor_commands: CyclerState<MotorCommands<f32>, "last_executed_motor_command">,

    motion_safe_exits_output: AdditionalOutput<MotionSafeExits, "motion_safe_exits_output">,
    actuated_motor_commands: AdditionalOutput<MotorCommands<f32>, "last_executed_motor_command">,

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
        let motor_commands = context.motor_commands;

        context
            .hardware_interface
            .write_to_actuators(
                motor_commands.positions,
                motor_commands.stiffnesses,
                *context.leds,
            )
            .wrap_err("failed to write to actuators")?;

        context.last_executed_motor_commands.positions = motor_commands.positions;
        context.last_executed_motor_commands.stiffnesses = motor_commands.stiffnesses;

        context
            .actuated_motor_commands
            .fill_if_subscribed(|| *motor_commands);
        context
            .motion_safe_exits_output
            .fill_if_subscribed(|| context.motion_safe_exits.clone());

        Ok(MainOutputs {})
    }
}
