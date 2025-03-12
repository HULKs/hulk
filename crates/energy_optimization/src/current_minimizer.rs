use filtering::hysteresis::less_than_with_absolute_hysteresis;
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
pub struct CurrentMinimizer {
    minimum_reached: bool,
    position_offset: Joints<f32>,
    parameters: CurrentMinimizerParameters,
}

impl CurrentMinimizer {
    pub fn optimize(
        &mut self,
        currents: Joints<f32>,
        positions: Joints<f32>,
        measured_positions: Joints<f32>,
        cycle_time: CycleTime,
        parameters: CurrentMinimizerParameters,
    ) -> Joints<f32> {
        self.parameters = parameters;

        // this optimization is inspired by the approach of Berlin United in their team research report 2019.
        let (joint, maximal_current) = currents
            .enumerate()
            .max_by(|(_, left), (_, right)| f32::total_cmp(left, right))
            .expect("currents must not be empty.");

        self.minimum_reached = less_than_with_absolute_hysteresis(
            self.minimum_reached,
            maximal_current,
            self.parameters.allowed_current..=self.parameters.allowed_current_upper_threshold,
        );

        if !self.minimum_reached {
            let max_adjustment =
                self.parameters.optimization_speed / cycle_time.last_cycle_duration.as_secs_f32();
            self.position_offset[joint] += (positions[joint] - measured_positions[joint])
                .clamp(-max_adjustment, max_adjustment)
        }

        positions + self.position_offset
    }

    pub fn reset(&mut self) {
        self.position_offset = Joints::fill(0.0);
    }
}
