use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    joints::{ArmJoints, Joints, JointsCommand},
    motion_selection::MotionSelection,
    parameters::MotorCommandOptimizerParameters,
    sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct MotorCommandOptimizer {
    position_offset: Joints<f32>,
    is_resetting: bool,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub motor_commands: Input<JointsCommand<f32>, "motor_commands">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,

    pub parameters:
        Parameter<MotorCommandOptimizerParameters, "motor_command_optimizer_parameters">,

    pub squared_position_offset_sum:
        AdditionalOutput<f32, "motor_position_optimization_offset_squared_sum">,
    pub position_offset: AdditionalOutput<Joints<f32>, "motor_position_optimization_offset">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub optimized_motor_commands: MainOutput<JointsCommand<f32>>,
}

impl MotorCommandOptimizer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            position_offset: Joints::default(),
            is_resetting: false,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let current_motion = context.motion_selection.current_motion;

        let optimization_forbidden = current_motion
            != types::motion_selection::MotionType::Penalized
            && current_motion != types::motion_selection::MotionType::Stand;

        let currents = context.sensor_data.currents;
        let commands = *context.motor_commands;
        let parameters = context.parameters;

        let squared_position_offset_sum: f32 = self
            .position_offset
            .as_flat_iter()
            .map(|position| position.powf(2.0))
            .sum();

        if squared_position_offset_sum > parameters.offset_reset_threshold || optimization_forbidden
        {
            self.is_resetting = true;
        }

        if self.is_resetting {
            if squared_position_offset_sum
                < parameters.offset_reset_threshold / parameters.offset_reset_offset
                && !optimization_forbidden
            {
                self.is_resetting = false;
            } else {
                self.position_offset = self.position_offset / parameters.offset_reset_speed;
            }
        }

        let maximal_current = currents.as_flat_iter().fold(0.0, f32::max);
        let reset_threshold_reached = maximal_current >= parameters.optimization_current_threshold;

        if reset_threshold_reached && !self.is_resetting {
            let position_offset = parameters
                .optimization_sign
                .as_flat_iter()
                .zip(currents.as_flat_iter())
                .map(|(correction_direction, current)| {
                    if current < maximal_current {
                        0.0
                    } else {
                        parameters.optimization_speed * correction_direction as f32
                    }
                });
            self.position_offset = self.position_offset + Joints::from_iter(position_offset);
        }

        let optimized_stiffnesses = Joints {
            left_arm: ArmJoints {
                hand: 0.0,
                ..commands.stiffnesses.left_arm
            },
            right_arm: ArmJoints {
                hand: 0.0,
                ..commands.stiffnesses.right_arm
            },
            ..commands.stiffnesses
        };

        let optimized_commands = JointsCommand {
            positions: commands.positions + self.position_offset,
            stiffnesses: optimized_stiffnesses,
        };

        context
            .squared_position_offset_sum
            .fill_if_subscribed(|| squared_position_offset_sum);
        context
            .position_offset
            .fill_if_subscribed(|| self.position_offset);

        Ok(MainOutputs {
            optimized_motor_commands: optimized_commands.into(),
        })
    }
}
