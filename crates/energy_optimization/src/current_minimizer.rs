use filtering::hysteresis::less_than_with_hysteresis_from_thresholds;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{cycle_time::CycleTime, joints::Joints};

use crate::parameters::CurrentMinimizerParameters;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
enum State {
    #[default]
    Optimizing,
    Resetting,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct CurrentMinimizer {
    minimum_reached: bool,
    position_offset: Joints<f32>,
    state: State,
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

        let squared_position_offset_sum: f32 = self
            .position_offset
            .into_iter()
            .map(|position| position.powf(2.0))
            .sum();

        if squared_position_offset_sum >= self.parameters.reset_threshold {
            self.state = State::Resetting;
        }

        match self.state {
            State::Optimizing => {
                // this optimization is inspired by the approach of Berlin United in their team research report 2019.
                let (joint, maximal_current) = currents
                    .enumerate()
                    .max_by(|(_, left), (_, right)| f32::total_cmp(left, right))
                    .expect("currents must not be empty.");

                self.minimum_reached = less_than_with_hysteresis_from_thresholds(
                    self.minimum_reached,
                    maximal_current,
                    self.parameters.allowed_current,
                    self.parameters.allowed_current_upper_threshold,
                );
                if !self.minimum_reached {
                    self.position_offset[joint] += self.parameters.optimization_sign[joint]
                        * self.parameters.optimization_speed
                        / cycle_time.last_cycle_duration.as_secs_f32();
                }
            }
            State::Resetting => {
                let resetting_finished =
                    squared_position_offset_sum < self.parameters.reset_base_offset;

                if resetting_finished {
                    self.state = State::Optimizing;
                } else {
                    self.position_offset = self.position_offset
                        / (1.0
                            + self.parameters.reset_speed
                                / cycle_time.last_cycle_duration.as_secs_f32());
                }
            }
        }

        positions + self.position_offset
    }

    pub fn reset(&mut self) {
        self.position_offset = Joints::fill(0.0);
    }
}
