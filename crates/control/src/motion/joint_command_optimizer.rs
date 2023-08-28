use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{collected_commands::CollectedCommands, joints::Joints, sensor_data::SensorData};

#[derive(Deserialize, Serialize)]
pub struct JointCommandOptimizer {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub collected_commands: Input<CollectedCommands, "collected_commands">,
    pub sensor_data: Input<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub optimized_commands: MainOutput<CollectedCommands>,
}

impl JointCommandOptimizer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let currents = context.sensor_data.current;
        let collected_commands = context.collected_commands;

        let maximal_current = collected_commands
            .positions
            .as_vec()
            .into_iter()
            .flatten()
            .fold(0.0, f32::max);

        let optimized_position_angles = collected_commands
            .positions
            .as_vec()
            .into_iter()
            .flatten()
            .zip(currents.as_vec().into_iter().flatten())
            .map(|(current, position)| {
                if current == maximal_current {
                    position + 0.1
                } else {
                    position
                }
            });

        let optimized_positions = Joints::from_iterator(optimized_position_angles);

        let optimized_commands = CollectedCommands {
            positions: optimized_positions,
            stiffnesses: collected_commands.stiffnesses,
        };

        Ok(MainOutputs {
            optimized_commands: optimized_commands.into(),
        })
    }
}
