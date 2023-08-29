use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::AdditionalOutput;
use hardware::ActuatorInterface;
use serde::{Deserialize, Serialize};
use types::{
    joints::{Joints, JointsCommand},
    led::Leds,
    motion_selection::MotionSafeExits,
};

#[derive(Deserialize, Serialize)]
pub struct CommandSender {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    final_positions: AdditionalOutput<Joints<f32>, "final_positions">,
    stiffnesses: AdditionalOutput<Joints<f32>, "stiffnesses">,
    motion_safe_exits_output: AdditionalOutput<MotionSafeExits, "motion_safe_exits_output">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
    optimized_motor_commands: Input<JointsCommand<f32>, "optimized_motor_commands">,
    leds: Input<Leds, "leds">,

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
        let optimized_motor_commands = context.optimized_motor_commands;

        // The actuators uses the raw sensor data (not corrected like current_positions) in their feedback loops,
        // thus the compensation is required to make them reach the actual desired position.
        context
            .hardware_interface
            .write_to_actuators(
                optimized_motor_commands.positions,
                optimized_motor_commands.stiffnesses,
                *context.leds,
            )
            .wrap_err("failed to write to actuators")?;

        context
            .final_positions
            .fill_if_subscribed(|| optimized_motor_commands.positions);
        context
            .stiffnesses
            .fill_if_subscribed(|| optimized_motor_commands.stiffnesses);
        context
            .motion_safe_exits_output
            .fill_if_subscribed(|| context.motion_safe_exits.clone());

        Ok(MainOutputs {})
    }
}
