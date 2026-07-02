use factrs::{containers::FactorBuilder, traits::Optimizer};

use crate::{
    factors::foot_above_ground::{FootHeightMeasurement, IntervalFootAboveGroundFactor},
    symbols::State,
};

use super::VinsBackend;

impl VinsBackend {
    pub(super) fn ingest_foot_heights(&mut self, foot_heights: Vec<FootHeightMeasurement>) {
        let Some(last) = foot_heights.last() else {
            return;
        };
        self.update_last_knot_time(last.time);

        let interval_groups = self.interval_groups(foot_heights, |measurement| measurement.time);

        for group in interval_groups {
            if !self.prepare_interval_for_measurements(group.start_index, "foot height") {
                continue;
            }

            let keys = (State(group.start_index), State(group.start_index + 1));
            let graph = self.optimizer.graph_mut();
            if let Some(factor) = graph
                .factors_for_residual_mut::<IntervalFootAboveGroundFactor, _>(keys)
                .next()
            {
                factor
                    .residual_as_mut::<IntervalFootAboveGroundFactor>()
                    .expect("factor query must return matching residual")
                    .extend_measurements(group.measurements);
            } else {
                let residual = IntervalFootAboveGroundFactor::new(
                    group.measurements,
                    group.start_time,
                    group.end_time,
                    self.config.foot_ground_sigma,
                );
                let factor = FactorBuilder::new(residual, keys).build();

                graph.add_factor(factor);
            }
        }
    }
}
