use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    joints::{Joints, JointsCommand},
    sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct MotorCommandOptimizer {
    original_motor_commands: JointsCommand<f32>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub motor_commands: Input<JointsCommand<f32>, "motor_commands">,
    pub sensor_data: Input<SensorData, "sensor_data">,

    pub motor_position_quadratic_deviation:
        AdditionalOutput<f32, "motor_position_quadratic_deviation">,
    pub motor_position_deviation_threshold: Parameter<f32, "motor_position_deviation_threshold">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub optimized_motor_commands: MainOutput<JointsCommand<f32>>,
}

impl MotorCommandOptimizer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            original_motor_commands: JointsCommand::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let currents = context.sensor_data.currents;
        let motor_commands = *context.motor_commands;

        let quadratic_deviation: f32 = (self.original_motor_commands.positions
            - motor_commands.positions)
            .as_vec()
            .into_iter()
            .flatten()
            .map(|position| position.powf(2.0))
            .sum();

        if quadratic_deviation > *context.motor_position_deviation_threshold {
            self.original_motor_commands = motor_commands;
        }

        let maximal_current = currents.as_vec().into_iter().flatten().fold(0.0, f32::max);

        let optimized_position_angles = self
            .original_motor_commands
            .positions
            .as_vec()
            .into_iter()
            .flatten()
            .zip(currents.as_vec().into_iter().flatten())
            .map(|(position, current)| {
                if current == maximal_current {
                    position // + 0.1 // todo correct in correct direction
                } else {
                    position
                }
            });

        let optimized_positions = Joints::from_iter(optimized_position_angles);

        let optimized_motor_commands = JointsCommand {
            positions: optimized_positions,
            stiffnesses: motor_commands.stiffnesses,
        };

        context
            .motor_position_quadratic_deviation
            .fill_if_subscribed(|| quadratic_deviation);

        Ok(MainOutputs {
            optimized_motor_commands: optimized_motor_commands.into(),
        })
    }
}
