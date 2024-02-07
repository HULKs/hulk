use std::f32::EPSILON;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{cycle_time::CycleTime, joints::Joints, motor_commands::MotorCommands};

use crate::parameters::CurrentMinimizerParameters;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
enum State {
    #[default]
    Optimizing,
    Resetting,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct CurrentMinimizer {
    position_offset: Joints<f32>,
    state: State,
    last_motor_commands: MotorCommands<Joints<f32>>,
    last_positions: Joints<f32>,
    parameters: CurrentMinimizerParameters,
}

impl CurrentMinimizer {
    pub fn optimize(
        &mut self,
        currents: Joints<f32>,
        positions: Joints<f32>,
        cycle_time: CycleTime,
        parameters: CurrentMinimizerParameters,
    ) -> Joints<f32> {
        self.parameters = parameters;

        let positions_difference = positions - self.last_positions;
        let squared_positions_difference_sum: f32 = positions_difference
            .into_iter()
            .map(|position| position.powf(2.0))
            .sum();

        let optimization_enabled = squared_positions_difference_sum
            < self.parameters.position_difference_threshold + EPSILON;

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
                    .expect("Currents should not be empty.");

                let minimum_reached = maximal_current <= self.parameters.allowed_current;
                if !minimum_reached {
                    self.position_offset[joint] += self.parameters.optimization_sign[joint]
                        * self.parameters.optimization_speed
                        / cycle_time.last_cycle_duration.as_secs_f32();
                }
            }
            State::Resetting => {
                let resetting_finished =
                    squared_position_offset_sum < self.parameters.reset_base_offset;

                if resetting_finished && optimization_enabled {
                    self.state = State::Optimizing;
                } else {
                    self.position_offset = self.position_offset
                        / (1.0
                            + self.parameters.reset_speed
                                / cycle_time.last_cycle_duration.as_secs_f32());
                }
            }
        }

        self.last_positions = positions;
        positions + self.position_offset
    }

    pub fn reset(&mut self) {
        self.position_offset = Joints::fill(0.0);
    }
}
