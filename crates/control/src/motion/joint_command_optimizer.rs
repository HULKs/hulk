use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{collected_commands::CollectedCommands, Joints, SensorData};

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
            .collect::<Vec<f32>>()
            .into_iter()
            .fold(0.0, f32::max);

        let optimized_position_angles: [f32; 26] = collected_commands
            .positions
            .as_vec()
            .into_iter()
            .flatten()
            .collect::<Vec<f32>>()
            .iter()
            .zip(
                currents
                    .as_vec()
                    .into_iter()
                    .flatten()
                    .collect::<Vec<f32>>()
                    .iter(),
            )
            .map(|(current, position)| {
                if *current == maximal_current {
                    *position + 0.1
                } else {
                    *position
                }
            })
            .collect::<Vec<f32>>()
            .try_into()
            .unwrap_or_else(|v: Vec<f32>| {
                panic!("Expected 26 joints but found {} values", v.len())
            });

        let optimized_positions = Joints::from_angles(optimized_position_angles);

        let optimized_commands = CollectedCommands {
            positions: optimized_positions.into(),
            compensated_positions: collected_commands.compensated_positions,
            stiffnesses: collected_commands.stiffnesses,
            leds: collected_commands.leds,
        };

        Ok(MainOutputs {
            optimized_commands: optimized_commands.into(),
        })
    }
}
