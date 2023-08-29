use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{
    joints::{Joints, JointsCommand},
    sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct MotorCommandOptimizer {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub motor_commands: Input<JointsCommand<f32>, "motor_commands">,
    pub sensor_data: Input<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub optimized_motor_commands: MainOutput<JointsCommand<f32>>,
}

impl MotorCommandOptimizer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let currents = context.sensor_data.currents;
        let motor_commands = context.motor_commands;

        let maximal_current = currents.as_vec().into_iter().flatten().fold(0.0, f32::max);

        let optimized_position_angles = motor_commands
            .positions
            .as_vec()
            .into_iter()
            .flatten()
            .zip(currents.as_vec().into_iter().flatten())
            .map(|(position, current)| {
                if current == maximal_current {
                    position + 0.1
                } else {
                    position
                }
            });

        let optimized_positions = Joints::from_iterator(optimized_position_angles);

        let optimized_motor_commands = JointsCommand {
            positions: optimized_positions,
            stiffnesses: motor_commands.stiffnesses,
        };

        Ok(MainOutputs {
            optimized_motor_commands: optimized_motor_commands.into(),
        })
    }
}
