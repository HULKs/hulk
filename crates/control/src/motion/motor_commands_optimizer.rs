use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{joints::Joints, motor_commands::MotorCommands};

#[derive(Deserialize, Serialize)]
pub struct MotorCommandsOptimizer {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motor_commands: Input<MotorCommands<Joints<f32>>, "motor_commands">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub optimized_motor_commands: MainOutput<MotorCommands<Joints<f32>>>,
}

impl MotorCommandsOptimizer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }
    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let mut motor_commands = *context.motor_commands;
        motor_commands.stiffnesses.left_arm.hand = 0.0;
        motor_commands.stiffnesses.right_arm.hand = 0.0;

        Ok(MainOutputs {
            optimized_motor_commands: motor_commands.into(),
        })
    }
}
