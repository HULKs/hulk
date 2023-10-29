use std::f32::EPSILON;

use serde::{Deserialize, Serialize};
use types::{
    joints::Joints, motor_commands::MotorCommands, parameters::CurrentMinimizerParameters,
};

#[derive(Default, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum State {
    #[default]
    Optimizing,
    Resetting,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Default)]
pub struct CurrentMinimizer {
    pub position_offset: Joints<f32>,
    pub state: State,
    pub last_motor_commands: MotorCommands<Joints<f32>>,
    pub parameters: CurrentMinimizerParameters,
}

impl CurrentMinimizer {
    fn optimize(
        mut self,
        currents: Joints<f32>,
        motor_commands: MotorCommands<Joints<f32>>,
    ) -> MotorCommands<Joints<f32>> {
        let motor_commands_position_difference =
            motor_commands.positions - self.last_motor_commands.positions;
        let squared_motor_commands_position_difference_sum: f32 =
            motor_commands_position_difference
                .into_iter()
                .map(|position| position.powf(2.0))
                .sum();

        let optimization_enabled = squared_motor_commands_position_difference_sum
            < self.parameters.motor_command_position_difference_threshold + EPSILON;

        let squared_position_offset_sum: f32 = self
            .position_offset
            .into_iter()
            .map(|position| position.powf(2.0))
            .sum();

        if squared_position_offset_sum >= self.parameters.reset_threshold || !optimization_enabled {
            self.state = State::Resetting;
        }

        match self.state {
            State::Optimizing => {
                // this optimization is inspired by the approach of Berlin United in their team research report 2019.
                let (joint, maximal_current) = currents
                    .enumerate()
                    .max_by(|(_, left), (_, right)| f32::total_cmp(left, right))
                    .unwrap();

                let minimum_not_reached =
                    maximal_current > self.parameters.allowed_current_threshold;
                if minimum_not_reached {
                    self.position_offset[joint] += self.parameters.optimization_sign[joint]
                        * self.parameters.optimization_speed_factor;
                }
            }
            State::Resetting => {
                let resetting_finished =
                    squared_position_offset_sum < self.parameters.reset_base_offset;

                if resetting_finished && optimization_enabled {
                    self.state = State::Optimizing;
                } else {
                    self.position_offset =
                        self.position_offset / (1.0 + self.parameters.reset_speed_factor);
                }
            }
        }

        let optimized_motor_commands = MotorCommands {
            positions: motor_commands.positions + self.position_offset,
            stiffnesses: motor_commands.stiffnesses,
        };

        self.last_motor_commands = motor_commands;

        optimized_motor_commands
    }
}
