use std::time::{Duration, SystemTime};

use factrs::{
    containers::FactorBuilder,
    core::{Huber, SE3},
    linalg::VectorX,
    traits::{Optimizer, Variable},
};

use crate::{
    factors::visual_odometry::{
        AdjacentVisualOdometryFactor, VisualOdometryDelta, VisualOdometryFactor,
        VisualOdometryMeasurement,
    },
    symbols::State,
    tau,
};

use super::VinsBackend;

const VISUAL_ODOMETRY_HUBER_THRESHOLD: f64 = 2.0;

impl VinsBackend {
    pub(super) fn ingest_visual_odometry(
        &mut self,
        visual_odometry: Vec<VisualOdometryMeasurement>,
    ) {
        let Some(last) = visual_odometry.last() else {
            return;
        };
        let last_timestamp = last.current_time;

        let mut same_interval_deltas = Vec::new();
        let mut adjacent_interval_deltas = Vec::new();
        for measurement in visual_odometry {
            for measurement in self.split_visual_odometry_measurement(measurement) {
                match self.visual_odometry_delta(measurement) {
                    Some(TimedVisualOdometryDelta::SameInterval(delta)) => {
                        same_interval_deltas.push(delta);
                    }
                    Some(TimedVisualOdometryDelta::AdjacentInterval(delta)) => {
                        adjacent_interval_deltas.push(delta);
                    }
                    None => {}
                }
            }
        }

        if same_interval_deltas.is_empty() && adjacent_interval_deltas.is_empty() {
            return;
        }
        self.update_last_knot_time(last_timestamp);
        self.add_visual_odometry_factors(same_interval_deltas);
        self.add_adjacent_visual_odometry_factors(adjacent_interval_deltas);
    }

    fn add_visual_odometry_factors(&mut self, deltas: Vec<IntervalVisualOdometryDelta>) {
        let interval_groups =
            self.interval_groups(deltas, |measurement| measurement.interval_start_time);

        for group in interval_groups {
            if !self.prepare_visual_odometry_states(group.start_index, group.start_index) {
                continue;
            }

            let keys = (State(group.start_index), State(group.start_index + 1));
            let graph = self.optimizer.graph_mut();
            // factrs robust kernels are factor-wide, so keep each 6D delta in
            // its own factor to avoid downweighting unrelated residuals.
            for measurement in group.measurements {
                let residual = VisualOdometryFactor::new(
                    vec![measurement.delta],
                    self.config.visual_odometry_noise,
                    self.config.knot_spacing.as_secs_f64(),
                );
                let factor = FactorBuilder::new(residual, keys)
                    .robust(Huber::new(VISUAL_ODOMETRY_HUBER_THRESHOLD))
                    .build();

                graph.add_factor(factor);
            }
        }
    }

    fn add_adjacent_visual_odometry_factors(&mut self, deltas: Vec<IntervalVisualOdometryDelta>) {
        let interval_groups =
            self.interval_groups(deltas, |measurement| measurement.interval_start_time);

        for group in interval_groups {
            let end_index = group.start_index + 1;
            if !self.prepare_visual_odometry_states(group.start_index, end_index) {
                continue;
            }

            let keys = (
                State(group.start_index),
                State(group.start_index + 1),
                State(group.start_index + 2),
            );
            let graph = self.optimizer.graph_mut();
            // factrs robust kernels are factor-wide, so keep each 6D delta in
            // its own factor to avoid downweighting unrelated residuals.
            for measurement in group.measurements {
                let residual = AdjacentVisualOdometryFactor::new(
                    vec![measurement.delta],
                    self.config.visual_odometry_noise,
                    self.config.knot_spacing.as_secs_f64(),
                );
                let factor = FactorBuilder::new(residual, keys)
                    .robust(Huber::new(VISUAL_ODOMETRY_HUBER_THRESHOLD))
                    .build();

                graph.add_factor(factor);
            }
        }
    }

    fn prepare_visual_odometry_states(&mut self, start_index: u32, end_index: u32) -> bool {
        self.init_intervals_through(end_index);
        if (start_index..=(end_index + 1)).all(|index| self.values.get(State(index)).is_some()) {
            return true;
        }

        log::debug!(
            "skipping visual odometry measurements for marginalized states {start_index}..{}",
            end_index + 1
        );
        false
    }

    fn visual_odometry_delta(
        &self,
        measurement: VisualOdometryMeasurement,
    ) -> Option<TimedVisualOdometryDelta> {
        if measurement.current_time <= measurement.previous_time {
            log::debug!("dropping non-forward visual odometry measurement");
            return None;
        }

        let intervals = self.visual_odometry_intervals(&measurement)?;

        match intervals.spanned_intervals {
            0 => Some(TimedVisualOdometryDelta::SameInterval(
                IntervalVisualOdometryDelta {
                    interval_start_time: intervals.previous_start_time,
                    delta: VisualOdometryDelta::from_measurement(
                        measurement,
                        intervals.previous_start_time,
                        intervals.previous_start_time + self.config.knot_spacing,
                    ),
                },
            )),
            1 => Some(TimedVisualOdometryDelta::AdjacentInterval(
                IntervalVisualOdometryDelta {
                    interval_start_time: intervals.previous_start_time,
                    delta: VisualOdometryDelta::new(
                        tau(
                            intervals.previous_start_time,
                            intervals.previous_start_time + self.config.knot_spacing,
                            measurement.previous_time,
                        ),
                        tau(
                            intervals.current_start_time,
                            intervals.current_start_time + self.config.knot_spacing,
                            measurement.current_time,
                        ),
                        measurement.robot_delta,
                    ),
                },
            )),
            skipped_intervals => {
                log::debug!(
                    "dropping visual odometry measurement spanning {skipped_intervals} intervals"
                );
                None
            }
        }
    }

    fn visual_odometry_intervals(
        &self,
        measurement: &VisualOdometryMeasurement,
    ) -> Option<VisualOdometryIntervals> {
        let previous_start_time = self
            .interval_assigner
            .current_or_initialize_interval_start_time(measurement.previous_time)?;
        let previous_index = self
            .interval_assigner
            .assign_or_initialize_interval(previous_start_time)?;
        let current_start_time = self
            .interval_assigner
            .current_or_initialize_interval_start_time(measurement.current_time)?;
        let current_index = self
            .interval_assigner
            .assign_or_initialize_interval(current_start_time)?;
        let spanned_intervals = current_index.checked_sub(previous_index)?;

        Some(VisualOdometryIntervals {
            previous_index,
            current_index,
            previous_start_time,
            current_start_time,
            spanned_intervals,
        })
    }

    fn split_visual_odometry_measurement(
        &self,
        measurement: VisualOdometryMeasurement,
    ) -> Vec<VisualOdometryMeasurement> {
        if measurement.current_time <= measurement.previous_time {
            return vec![measurement];
        }

        let Some(intervals) = self.visual_odometry_intervals(&measurement) else {
            return vec![measurement];
        };
        if intervals.spanned_intervals == 0 {
            return vec![measurement];
        }

        let Ok(total_duration) = measurement
            .current_time
            .duration_since(measurement.previous_time)
        else {
            return vec![measurement];
        };
        if total_duration.is_zero() {
            return vec![measurement];
        }

        let tangent = measurement.robot_delta.log();
        let mut segments = Vec::new();
        let mut segment_start_time = measurement.previous_time;
        for boundary_index in (intervals.previous_index + 1)..=intervals.current_index {
            let Some(boundary_time) = self.interval_assigner.interval_start_time(boundary_index)
            else {
                continue;
            };
            if boundary_time <= segment_start_time || boundary_time >= measurement.current_time {
                continue;
            }
            let guarded_boundary_time = boundary_time
                .checked_sub(Duration::from_nanos(1))
                .unwrap_or(boundary_time);
            if guarded_boundary_time > segment_start_time {
                segments.push(split_visual_odometry_segment(
                    segment_start_time,
                    guarded_boundary_time,
                    total_duration,
                    &tangent,
                ));
            }
            segment_start_time = boundary_time;
        }
        if measurement.current_time > segment_start_time {
            segments.push(split_visual_odometry_segment(
                segment_start_time,
                measurement.current_time,
                total_duration,
                &tangent,
            ));
        }

        segments
    }
}

fn split_visual_odometry_segment(
    previous_time: SystemTime,
    current_time: SystemTime,
    total_duration: Duration,
    tangent: &VectorX,
) -> VisualOdometryMeasurement {
    let segment_duration = current_time
        .duration_since(previous_time)
        .expect("split segment times must be ordered");
    let fraction = segment_duration.as_secs_f64() / total_duration.as_secs_f64();
    let scaled_tangent = tangent * fraction;
    VisualOdometryMeasurement {
        previous_time,
        current_time,
        robot_delta: SE3::exp(scaled_tangent.as_view()),
    }
}

struct VisualOdometryIntervals {
    previous_index: u32,
    current_index: u32,
    previous_start_time: SystemTime,
    current_start_time: SystemTime,
    spanned_intervals: u32,
}

enum TimedVisualOdometryDelta {
    SameInterval(IntervalVisualOdometryDelta),
    AdjacentInterval(IntervalVisualOdometryDelta),
}

struct IntervalVisualOdometryDelta {
    interval_start_time: SystemTime,
    delta: VisualOdometryDelta,
}
