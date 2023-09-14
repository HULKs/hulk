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
    motor_commands_residual: JointsCommand<f32>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub motor_commands: Input<JointsCommand<f32>, "motor_commands">,
    pub sensor_data: Input<SensorData, "sensor_data">,

    pub motor_position_quadratic_deviation:
        AdditionalOutput<f32, "motor_position_quadratic_deviation">,
    pub motor_commands_residual: AdditionalOutput<JointsCommand<f32>, "motor_commands_residual">,

    pub motor_position_deviation_threshold: Parameter<f32, "motor_position_deviation_threshold">,
    pub motor_position_optimization_step:
        Parameter<Joints<f32>, "motor_position_optimization_step">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub optimized_motor_commands: MainOutput<JointsCommand<f32>>,
}

impl MotorCommandOptimizer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            motor_commands_residual: JointsCommand::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let currents = context.sensor_data.currents;
        let motor_commands = *context.motor_commands;

        let quadratic_deviation: f32 = self
            .motor_commands_residual
            .positions
            .as_vec()
            .into_iter()
            .flatten()
            .map(|position| position.powf(2.0))
            .sum();

        if quadratic_deviation > *context.motor_position_deviation_threshold {
            self.motor_commands_residual = JointsCommand::default();
        }

        let maximal_current = currents.as_vec().into_iter().flatten().fold(0.0, f32::max);

        let optimized_position_angles = context
            .motor_position_optimization_step
            .as_vec()
            .into_iter()
            .flatten()
            .zip(currents.as_vec().into_iter().flatten())
            .map(|(correction, current)| {
                if current == maximal_current {
                    correction
                } else {
                    0.0
                }
            });

        if maximal_current >= 0.09 {
            self.motor_commands_residual.positions = self.motor_commands_residual.positions
                + Joints::from_iter(optimized_position_angles);
        }

        let optimized_motor_commands = JointsCommand {
            positions: motor_commands.positions + self.motor_commands_residual.positions,
            stiffnesses: motor_commands.stiffnesses,
        };

        context
            .motor_position_quadratic_deviation
            .fill_if_subscribed(|| quadratic_deviation);
        context
            .motor_commands_residual
            .fill_if_subscribed(|| self.motor_commands_residual);

        Ok(MainOutputs {
            optimized_motor_commands: optimized_motor_commands.into(),
        })
    }
}
