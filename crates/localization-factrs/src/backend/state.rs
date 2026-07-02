use std::time::SystemTime;

use factrs::{
    containers::FactorBuilder,
    core::{Graph, Values, Vector3},
    traits::Optimizer,
    variables::SE23,
};
use itertools::Itertools;

use crate::{
    factors::gaussian_process_prior::GaussianProcessPriorFactor, initial_state::InitialState,
    symbols::State,
};

use super::{
    BackendConfiguration, EMPTY_INTERVAL_PROCESS_COVARIANCE_SCALE, LONG_GAP_MIN_EMPTY_INTERVALS,
    VinsBackend, graph::add_field_containment_factor,
};

impl VinsBackend {
    pub(super) fn init_intervals_through(&mut self, interval_start_index: u32) {
        let first_missing_interval = self
            .highest_initialized_interval
            .map_or(0, |index| index + 1);
        if first_missing_interval > interval_start_index {
            return;
        }

        let is_long_gap = interval_start_index.saturating_sub(first_missing_interval)
            >= LONG_GAP_MIN_EMPTY_INTERVALS;
        if is_long_gap {
            reset_state_velocity(&mut self.values, State(first_missing_interval));
        }
        for index in first_missing_interval..=interval_start_index {
            init_interval_states(
                &mut self.values,
                self.optimizer.graph_mut(),
                &self.config,
                &self.initial_state,
                index,
                is_long_gap && index < interval_start_index,
            );
        }
        self.highest_initialized_interval = Some(interval_start_index);
    }

    pub(super) fn interval_states_available(&self, interval_start_index: u32) -> bool {
        self.values.get(State(interval_start_index)).is_some()
            && self.values.get(State(interval_start_index + 1)).is_some()
    }

    pub(super) fn update_last_knot_time(&mut self, time: SystemTime) {
        self.last_knot_time = Some(self.last_knot_time.map_or(time, |t| t.max(time)));
    }

    pub(super) fn interval_groups<T>(
        &self,
        measurements: Vec<T>,
        time_of: impl Fn(&T) -> SystemTime,
    ) -> Vec<IntervalGroup<T>> {
        let mut interval_groups = Vec::new();
        for (key, chunk) in measurements
            .into_iter()
            .chunk_by(|measurement| {
                self.interval_assigner
                    .current_or_initialize_interval_start_time(time_of(measurement))
            })
            .into_iter()
        {
            let Some(start_time) = key else {
                log::debug!("dropping measurements before earliest solver time");
                continue;
            };
            let Some(start_index) = self
                .interval_assigner
                .assign_or_initialize_interval(start_time)
            else {
                log::debug!("dropping measurements before earliest solver time");
                continue;
            };

            interval_groups.push(IntervalGroup {
                start_index,
                start_time,
                end_time: start_time + self.config.knot_spacing,
                measurements: chunk.collect(),
            });
        }

        interval_groups
    }

    pub(super) fn prepare_interval_for_measurements(
        &mut self,
        interval_start_index: u32,
        sensor_name: &str,
    ) -> bool {
        self.init_intervals_through(interval_start_index);
        if self.interval_states_available(interval_start_index) {
            return true;
        }

        log::debug!(
            "skipping {sensor_name} measurements for marginalized interval {interval_start_index}"
        );
        false
    }
}

pub(super) struct IntervalGroup<T> {
    pub(super) start_index: u32,
    pub(super) start_time: SystemTime,
    pub(super) end_time: SystemTime,
    pub(super) measurements: Vec<T>,
}

fn reset_state_velocity(values: &mut Values, state: State) {
    if let Some(pose) = values.get_mut(state) {
        *pose = zero_velocity_pose(pose);
    }
}

pub(super) fn zero_velocity_pose(pose: &SE23) -> SE23 {
    SE23::from_rot_vel_trans(
        pose.rot().clone(),
        Vector3::zeros(),
        pose.xyz().into_owned(),
    )
}

fn init_interval_states(
    values: &mut Values,
    graph: &mut Graph,
    config: &BackendConfiguration,
    initial_state: &InitialState,
    interval_start_index: u32,
    is_empty_bridge_interval: bool,
) {
    let start = State(interval_start_index);
    let end = State(interval_start_index + 1);

    if values.init_if_missing(&initial_state.pose, start, false) {
        add_field_containment_factor(graph, start, config);
    }
    if values.init_if_missing(&initial_state.pose, end, is_empty_bridge_interval) {
        add_field_containment_factor(graph, end, config);
        let gp_residual = if is_empty_bridge_interval {
            GaussianProcessPriorFactor::new_zero_start_velocity_bridge(
                config.knot_spacing.as_secs_f64(),
                &config.gyroscope_process_noise,
                &config.accelerometer_process_noise,
                EMPTY_INTERVAL_PROCESS_COVARIANCE_SCALE,
            )
        } else {
            GaussianProcessPriorFactor::new(
                config.knot_spacing.as_secs_f64(),
                &config.gyroscope_process_noise,
                &config.accelerometer_process_noise,
            )
        };

        let gp_factor = FactorBuilder::new(gp_residual, (start, end)).build();
        graph.add_factor(gp_factor);
    }
}

trait InitStateExt {
    fn init_if_missing(&mut self, initial: &SE23, index: State, reset_velocity: bool) -> bool;
}

impl InitStateExt for Values {
    fn init_if_missing(&mut self, initial: &SE23, index: State, reset_velocity: bool) -> bool {
        if self.get(index).is_some() {
            return false;
        }

        let previous = index
            .0
            .checked_sub(1)
            .and_then(|i| self.get(State(i)))
            .unwrap_or(initial);

        self.insert(
            index,
            if reset_velocity {
                zero_velocity_pose(previous)
            } else {
                previous.clone()
            },
        );
        true
    }
}
