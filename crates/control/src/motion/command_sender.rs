use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::AdditionalOutput;
use hardware::ActuatorInterface;
use serde::{Deserialize, Serialize};
use types::{
    joints::Joints, led::Leds, motion_selection::MotionSafeExits, motor_commands::MotorCommands,
};

#[derive(Deserialize, Serialize)]
pub struct CommandSender {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    optimized_motor_commands: Input<MotorCommands<Joints<f32>>, "optimized_motor_commands">,
    leds: Input<Leds, "leds">,
    joint_calibration_offsets: Parameter<Joints<f32>, "joint_calibration_offsets">,
    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
    last_actuated_motor_commands_before_offset:
        CyclerState<MotorCommands<Joints<f32>>, "last_actuated_motor_commands_before_offset">,

    motion_safe_exits_output: AdditionalOutput<MotionSafeExits, "motion_safe_exits_output">,
    actuated_motor_commands:
        AdditionalOutput<MotorCommands<Joints<f32>>, "actuated_motor_commands">,
    actuated_motor_commands_difference:
        AdditionalOutput<Joints<f32>, "actuated_motor_commands_difference">,

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
        let motor_commands = context.optimized_motor_commands;

        context
            .hardware_interface
            .write_to_actuators(
                motor_commands.positions + *context.joint_calibration_offsets,
                motor_commands.stiffnesses,
                *context.leds,
            )
            .wrap_err("failed to write to actuators")?;

        context
            .actuated_motor_commands
            .fill_if_subscribed(|| *motor_commands);
        context
            .motion_safe_exits_output
            .fill_if_subscribed(|| context.motion_safe_exits.clone());

        context
            .actuated_motor_commands_difference
            .fill_if_subscribed(|| {
                motor_commands.positions
                    - context.last_actuated_motor_commands_before_offset.positions
            });

        context.last_actuated_motor_commands_before_offset.positions = motor_commands.positions;
        context
            .last_actuated_motor_commands_before_offset
            .stiffnesses = motor_commands.stiffnesses;

        Ok(MainOutputs {})
    }
}
