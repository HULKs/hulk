use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::AdditionalOutput;
use hardware::ActuatorInterface;
use serde::{Deserialize, Serialize};
use types::{led::Leds, motion_selection::MotionSafeExits, motor_commands::MotorCommand};

#[derive(Deserialize, Serialize)]
pub struct CommandSender {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motor_commands: Input<MotorCommand<f32>, "motor_commands">,
    leds: Input<Leds, "leds">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
    hardware_interface: HardwareInterface,

    motion_safe_exits_output: AdditionalOutput<MotionSafeExits, "motion_safe_exits_output">,
    executed_motor_commands: AdditionalOutput<MotorCommand<f32>, "executed_motor_commands">,

    executed_motor_commands_state: CyclerState<MotorCommand<f32>, "executed_motor_commands">,
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

        context.executed_motor_commands_state.positions = motor_commands.positions;
        context.executed_motor_commands_state.stiffnesses = motor_commands.stiffnesses;

        context
            .executed_motor_commands
            .fill_if_subscribed(|| *motor_commands);
        context
            .motion_safe_exits_output
            .fill_if_subscribed(|| context.motion_safe_exits.clone());

        Ok(MainOutputs {})
    }
}
