use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{
    joints::Joints, motion_command::MotionCommand, motor_commands::MotorCommands,
    sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct Animation {
    saved_joint_values: Joints<f32>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    sensor_data: Input<SensorData, "sensor_data">,
    motion_command: Input<MotionCommand, "motion_command">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub animation_commands: MainOutput<MotorCommands<Joints<f32>>>,
}

impl Animation {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            saved_joint_values: Joints::default(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let animation_unstiff_command = MotorCommands {
            positions: context.sensor_data.positions,
            stiffnesses: Joints::fill(0.0),
        };
        let animation_stiff_command = MotorCommands {
            positions: self.saved_joint_values,
            stiffnesses: Joints::fill(1.0),
        };
        let output = match context.motion_command {
            MotionCommand::Animation { stiff: true } => animation_stiff_command,
            MotionCommand::Animation { stiff: false } => {
                self.saved_joint_values = context.sensor_data.positions;
                animation_unstiff_command
            }
            _ => Default::default(),
        };

        Ok(MainOutputs {
            animation_commands: framework::MainOutput { value: output },
        })
    }
}
